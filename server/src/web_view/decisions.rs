//! UI descriptions of the engine's pending requests. Selections are validated here;
//! previews never execute a response or advance the random stream.
use crate::action::Action;
use crate::card::{HandCard, validate_card_selection};
use crate::content::persistent_events::*;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernment, government_advances};
use crate::structure::Structure;
use crate::unit::validate_units_selection;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub(super) fn structure_name(s: &Structure) -> String {
    match s {
        Structure::CityCenter => "City center".into(),
        Structure::Building(b) => b.to_string(),
        Structure::Wonder(w) => w.name(),
    }
}

fn option(
    value: impl serde::Serialize,
    name: impl Into<String>,
    description: impl Into<String>,
    position: Option<crate::position::Position>,
) -> Value {
    json!({"value":value,"name":name.into(),"description":description.into(),"position":position})
}

fn resource_field(
    cost: &PaymentOptions,
    name: &str,
    optional: bool,
    available: &ResourcePile,
    reward: bool,
    stock: &ResourcePile,
) -> Value {
    // Resource keys come from the same serialized piles used by every action.
    let types = cost
        .possible_resource_types()
        .into_iter()
        .flat_map(|r| {
            serde_json::to_value(ResourcePile::of(r, 1))
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        })
        .unique()
        .collect::<Vec<_>>();
    json!({"name":name,"optional":optional,"resources":types,"initial":if reward {cost.default_payment()} else {cost.first_valid_payment(available).unwrap_or_default()},"cost":cost.default_payment(),
        "choices":payment_choices(cost, stock, reward, optional)})
}

// Present complete, engine-validated payments. Large choice spaces use direct
// amount selectors in the viewer instead of building an unbounded list.
fn payment_choices(cost: &PaymentOptions, stock: &ResourcePile, reward: bool, optional: bool) -> Option<Vec<ResourcePile>> {
    use crate::resource::ResourceType;
    fn visit(cost: &PaymentOptions, stock: &ResourcePile, reward: bool, types: &[ResourceType], remaining: u8,
        pile: ResourcePile, result: &mut Vec<ResourcePile>, budget: &mut usize) -> bool {
        if *budget == 0 || result.len() > 128 { return false; }
        *budget -= 1;
        let Some((resource, rest)) = types.split_first() else {
            if cost.is_valid_payment(&pile) { result.push(pile); }
            return true;
        };
        let max = if reward { remaining } else { remaining.min(stock.get(resource)) };
        for amount in 0..=max {
            let mut next = pile.clone();
            next.add_type(*resource, i32::from(amount));
            if !visit(cost, stock, reward, rest, remaining - amount, next, result, budget) { return false; }
        }
        true
    }
    // Expanding conversions need the unrestricted amount picker rather than a
    // total capped at the printed cost.
    if cost.conversions.iter().any(|c| c.from.iter().any(|from| c.to.amount() > from.amount())) { return None; }
    let mut types = cost.possible_resource_types();
    types.sort(); types.dedup();
    let mut result = vec![];
    if !visit(cost, stock, reward, &types, cost.default_payment().amount(), ResourcePile::empty(), &mut result, &mut 10_000) { return None; }
    if optional && !result.iter().any(ResourcePile::is_empty) { result.push(ResourcePile::empty()); }
    Some(result)
}

pub(super) fn describe(game: &Game, seat: usize) -> Option<Value> {
    if seat != game.active_player() {
        return None;
    }
    let h = game.current_event_handler()?;
    if h.response.is_some() {
        return None;
    }
    let mut title = h.origin.name(game);
    let mut description = String::new();
    let mut min = 1;
    let mut max = 1;
    let mut fields = vec![];
    let p = game.player(seat);
    let options = match &h.request {
        PersistentEventRequest::ExploreResolution => return None,
        PersistentEventRequest::BoolRequest(_) => return None,
        PersistentEventRequest::ResourceReward(r)
            if r.reward.payment_options.default.amount() == 1 =>
        {
            return None;
        }
        PersistentEventRequest::Payment(requests) => {
            if requests.iter().any(|r| r.name == "Pay to gain the Action Card")
                && let crate::events::EventOrigin::Incident(id) = h.origin
                && let Some(card) = &game.cache.get_incident(id).action_card
            {
                let text = card.civil_card.description.strip_prefix(
                    crate::content::incidents::great_persons::GREAT_PERSON_DESCRIPTION
                ).unwrap_or(&card.civil_card.description).trim();
                description = format!("When played ({}): {text}",
                    if card.civil_card.action_type.free { "free action" } else { "1 action" });
            }
            let mut available = p.resources.clone();
            for r in requests {
                let field = resource_field(&r.cost, &r.name, r.optional, &available, false, &p.resources);
                let initial: ResourcePile =
                    serde_json::from_value(field["initial"].clone()).unwrap();
                available -= initial;
                fields.push(field);
            }
            min = 0;
            max = 0;
            vec![]
        }
        PersistentEventRequest::ResourceReward(r) => {
            title = r.name.clone();
            fields.push(resource_field(
                &r.reward.payment_options,
                &r.name,
                false,
                &p.resources,
                true,
                &p.resources,
            ));
            min = 0;
            max = 0;
            vec![]
        }
        PersistentEventRequest::SelectAdvance(r) => {
            description = "Choose an advance".into();
            r.choices
                .iter()
                .map(|a| {
                    option(
                        a,
                        a.name(game),
                        &game.cache.get_advance(*a).description,
                        None,
                    )
                })
                .collect()
        }
        PersistentEventRequest::SelectPlayer(r) => {
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|i| option(i, &game.player(*i).civilization.name, "", None))
                .collect()
        }
        PersistentEventRequest::SelectUnitType(r) => {
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|u| option(u, u.name(game), "", None))
                .collect()
        }
        PersistentEventRequest::SelectPositions(r) => {
            min = *r.needed.start();
            max = *r.needed.end();
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|pos| {
                    let mut o = option(pos, pos.to_string(), "", Some(*pos));
                    o["terrain"] = json!(game.map.get(*pos));
                    o
                })
                .collect()
        }
        PersistentEventRequest::SelectUnits(r) => {
            min = *r.request.needed.start();
            max = *r.request.needed.end();
            description.clone_from(&r.request.description);
            r.request
                .choices
                .iter()
                .map(|id| {
                    let u = game.player(r.player).get_unit(*id);
                    option(
                        id,
                        format!("{} #{} · {}", u.unit_type.name(game), id + 1, u.position),
                        "",
                        Some(u.position),
                    )
                })
                .collect()
        }
        PersistentEventRequest::SelectStructures(r) => {
            min = *r.needed.start();
            max = *r.needed.end();
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|s| {
                    option(
                        s,
                        format!("{} · {}", structure_name(&s.structure), s.position),
                        "",
                        Some(s.position),
                    )
                })
                .collect()
        }
        PersistentEventRequest::SelectHandCards(r) => {
            // Objective claims already have their dedicated compact panel.
            if matches!(
                game.events.last()?.event_type,
                PersistentEventType::SelectObjectives(_)
            ) {
                return None;
            }
            min = *r.needed.start();
            max = *r.needed.end();
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|card| {
                    let text = match card {
                        HandCard::ActionCard(id) => {
                            let c = game.cache.get_action_card(*id);
                            format!(
                                "{}{}",
                                c.civil_card.description,
                                c.tactics_card
                                    .as_ref()
                                    .map(|t| format!("\n{}: {}", t.name, t.description))
                                    .unwrap_or_default()
                            )
                        }
                        HandCard::ObjectiveCard(id) => game
                            .cache
                            .get_objective_card(*id)
                            .objectives
                            .iter()
                            .map(|o| o.description.clone())
                            .join("\n"),
                        HandCard::Wonder(w) => w.info(game).description.clone(),
                    };
                    option(card, card.name(game), text, None)
                })
                .collect()
        }
        PersistentEventRequest::ChangeGovernment => {
            description = "Choose a government and its advances".into();
            government_choices(game, seat)
                .into_iter()
                .map(|c| {
                    let text = c
                        .additional_advances
                        .iter()
                        .map(|a| a.name(game))
                        .join(", ");
                    option(&c, c.new_government.clone(), text, None)
                })
                .collect()
        }
    };
    Some(
        json!({"name":title,"description":description,"min":min,"max":max,"options":options,"fields":fields,
        "reward":matches!(h.request,PersistentEventRequest::ResourceReward(_)),
        "advanceSelection":matches!(h.request,PersistentEventRequest::SelectAdvance(_)),
        "endOfAge":crate::status_phase::get_status_phase(game).is_some()}),
    )
}

fn government_choices(game: &Game, seat: usize) -> Vec<ChangeGovernment> {
    let p = game.player(seat);
    let count = government_advances(p, game).len().saturating_sub(1);
    game.cache
        .get_governments()
        .iter()
        .filter(|g| p.can_advance_ignore_contradicting(g.advances[0].advance, game))
        .flat_map(|g| {
            g.advances
                .iter()
                .skip(1)
                .map(|a| a.advance)
                .combinations(count)
                .map(|a| ChangeGovernment::new(g.name.clone(), a))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn selected<T: DeserializeOwned + PartialEq + Ord>(
    values: &Value,
    r: &MultiRequest<T>,
) -> Result<Vec<T>, String> {
    let choices: Vec<T> = serde_json::from_value(values.clone()).map_err(|e| e.to_string())?;
    if !r.is_valid(&choices)
        || choices.iter().any(|v| !r.choices.contains(v))
        || choices
            .iter()
            .enumerate()
            .any(|(i, v)| choices[..i].contains(v))
    {
        return Err(format!(
            "Select {}–{} choices",
            r.needed.start(),
            r.needed.end()
        ));
    }
    Ok(choices)
}

fn single<T: DeserializeOwned + PartialEq>(values: &Value, choices: &[T]) -> Result<T, String> {
    let mut v: Vec<T> = serde_json::from_value(values.clone()).map_err(|e| e.to_string())?;
    if v.len() != 1 || !choices.contains(&v[0]) {
        return Err("Select one choice".into());
    }
    Ok(v.remove(0))
}

pub(super) fn preview(game: &Game, seat: usize, input: &Value) -> Result<Value, String> {
    if seat != game.active_player() {
        return Err("Wait for your turn".into());
    }
    let h = game.current_event_handler().ok_or("No decision pending")?;
    let values = &input["values"];
    let piles = || {
        serde_json::from_value::<Vec<ResourcePile>>(input["payments"].clone())
            .map_err(|e| e.to_string())
    };
    let response = match &h.request {
        PersistentEventRequest::Payment(requests) => {
            let payments = piles()?;
            if payments.len() != requests.len() {
                return Err("Choose each payment".into());
            }
            let mut available = game.player(seat).resources.clone();
            for (r, p) in requests.iter().zip(&payments) {
                if !(r.optional && p.is_empty()) && !r.cost.is_valid_payment(p) {
                    return Err(format!("Choose a valid payment for {}", r.name));
                }
                if !available.has_at_least(p) {
                    return Err("Not enough resources".into());
                }
                available -= p.clone();
            }
            EventResponse::Payment(payments)
        }
        PersistentEventRequest::ResourceReward(r) => {
            let payments = piles()?;
            if payments.len() != 1 || !r.reward.payment_options.is_valid_payment(&payments[0]) {
                return Err("Choose the resources to gain".into());
            }
            EventResponse::ResourceReward(payments[0].clone())
        }
        PersistentEventRequest::SelectAdvance(r) => {
            EventResponse::SelectAdvance(single(values, &r.choices)?)
        }
        PersistentEventRequest::SelectPlayer(r) => {
            EventResponse::SelectPlayer(single(values, &r.choices)?)
        }
        PersistentEventRequest::SelectUnitType(r) => {
            EventResponse::SelectUnitType(single(values, &r.choices)?)
        }
        PersistentEventRequest::SelectPositions(r) => {
            EventResponse::SelectPositions(selected(values, r)?)
        }
        PersistentEventRequest::SelectUnits(r) => {
            let s = selected(values, &r.request)?;
            validate_units_selection(&s, game, game.player(r.player))?;
            EventResponse::SelectUnits(s)
        }
        PersistentEventRequest::SelectStructures(r) => {
            let mut s = selected(values, r)?;
            s.sort();
            if !is_selected_structures_valid(game, &s) {
                return Err("Select all pieces of a city before its city center".into());
            }
            EventResponse::SelectStructures(s)
        }
        PersistentEventRequest::SelectHandCards(r) => {
            let s = selected(values, r)?;
            validate_card_selection(&s, game)?;
            EventResponse::SelectHandCards(s)
        }
        PersistentEventRequest::ChangeGovernment => {
            EventResponse::ChangeGovernmentType(single(values, &government_choices(game, seat))?)
        }
        _ => return Err("Use the current decision controls".into()),
    };
    Ok(json!({"action":Action::Response(response)}))
}
