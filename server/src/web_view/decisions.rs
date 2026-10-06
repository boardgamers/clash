//! UI descriptions of the engine's pending requests. Selections are validated here;
//! previews never execute a response or advance the random stream.
use crate::action::Action;
use crate::card::{HandCard, validate_card_selection};
use crate::consts::MAX_HUMAN_PLAYERS;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::*;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::objective_card::ObjectiveType;
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernment, government_advances};
use crate::structure::Structure;
use crate::unit::validate_units_selection;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

// Advance requests either grant an advance, purchase it in a subsequent payment
// request, or borrow its effect. Keep these distinctions in the shared tree.
pub(super) fn advance_mode(game: &Game, handler: &PersistentEventHandler) -> &'static str {
    if let PersistentEventType::CustomAction(custom) = &game.current_event().event_type {
        match custom.action.action {
            CustomActionType::Scholar | CustomActionType::GoldenAge => return "paid",
            CustomActionType::GreatLibrary => return "borrow",
            _ => {}
        }
    }
    match handler.origin {
        EventOrigin::CivilCard(33 | 34 | 41 | 42) => "paid",
        _ => "free",
    }
}

// Card-granted advances leave the event counter alone, even when their research
// costs resources (e.g. Synergies). The Library only borrows an effect. Regular
// purchases, the status-phase advance, Dogma and leader research use a marker.
pub(super) fn advance_uses_event_marker(game: &Game, handler: &PersistentEventHandler) -> bool {
    !matches!(handler.origin, EventOrigin::CivilCard(_)) && advance_mode(game, handler) != "borrow"
}

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

fn great_seer_choice_description(game: &Game, handler: &PersistentEventHandler) -> Option<String> {
    if handler.origin != EventOrigin::CivilCard(158) {
        return None;
    }
    let order = MAX_HUMAN_PLAYERS.checked_sub(usize::try_from(handler.priority).ok()?)?;
    let players = game.human_players_sorted(game.current_event().player.index);
    let target = *players.get(order)?;
    let recipient = if target == game.active_player() {
        "your next objective card".to_string()
    } else {
        format!(
            "the next objective card for {}",
            game.player(target).civilization.name
        )
    };
    let others = players
        .iter()
        .skip(order + 1)
        .map(|p| game.player(*p).civilization.name.as_str())
        .collect_vec();
    let remaining = match others.as_slice() {
        [] => String::new(),
        [other] => format!(" The other card goes to {other} on their next objective draw."),
        _ => format!(
            " You will assign the remaining cards to {} for their next objective draws.",
            crate::utils::format_list(&others, "", "and")
        ),
    };
    Some(format!("Choose {recipient}.{remaining}"))
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
pub(super) fn payment_choices(
    cost: &PaymentOptions,
    stock: &ResourcePile,
    reward: bool,
    optional: bool,
) -> Option<Vec<ResourcePile>> {
    use crate::resource::ResourceType;
    fn visit(
        cost: &PaymentOptions,
        stock: &ResourcePile,
        reward: bool,
        types: &[ResourceType],
        remaining: u8,
        pile: ResourcePile,
        result: &mut Vec<ResourcePile>,
        budget: &mut usize,
    ) -> bool {
        if *budget == 0 || result.len() > 128 {
            return false;
        }
        *budget -= 1;
        let Some((resource, rest)) = types.split_first() else {
            if cost.is_valid_payment(&pile) {
                result.push(pile);
            }
            return true;
        };
        let max = if reward {
            remaining
        } else {
            remaining.min(stock.get(resource))
        };
        for amount in 0..=max {
            let mut next = pile.clone();
            next.add_type(*resource, i32::from(amount));
            if !visit(
                cost,
                stock,
                reward,
                rest,
                remaining - amount,
                next,
                result,
                budget,
            ) {
                return false;
            }
        }
        true
    }
    // Keep choice cards for bounded exchanges such as Alphabet (one route can
    // yield two ideas). Validate every candidate against the real conversion rules.
    let mut max_amount = u16::from(cost.default_payment().amount());
    for conversion in &cost.conversions {
        let from = conversion
            .from
            .iter()
            .map(ResourcePile::amount)
            .filter(|n| *n > 0)
            .min()
            .unwrap_or(1);
        let to = conversion.to.amount();
        if to > from {
            max_amount = max_amount
                .saturating_mul(u16::from(to))
                .div_ceil(u16::from(from));
        }
    }
    let Ok(max_amount) = u8::try_from(max_amount) else {
        return None;
    };
    let mut types = cost.possible_resource_types();
    types.sort();
    types.dedup();
    let mut result = vec![];
    if !visit(
        cost,
        stock,
        reward,
        &types,
        max_amount,
        ResourcePile::empty(),
        &mut result,
        &mut 10_000,
    ) {
        return None;
    }
    if optional && !result.iter().any(ResourcePile::is_empty) {
        result.push(ResourcePile::empty());
    }
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
    let p = game.player(game.current_event().player.index);
    let tactics_selection = matches!(&h.request, PersistentEventRequest::SelectHandCards(_))
        && (matches!(
            &h.origin,
            EventOrigin::Advance(crate::advance::Advance::Tactics)
        ) || matches!(&h.origin, EventOrigin::LeaderAbility(name) if name == "Loyalty"));
    let options = match &h.request {
        PersistentEventRequest::ExploreResolution => return None,
        PersistentEventRequest::BoolRequest(_) => return None,
        PersistentEventRequest::ResourceReward(r)
            if r.reward.payment_options.default.amount() == 1 =>
        {
            return None;
        }
        PersistentEventRequest::Payment(requests) => {
            if requests
                .iter()
                .any(|r| r.name == "Pay to gain the Action Card")
                && let crate::events::EventOrigin::Incident(id) = h.origin
                && let Some(card) = &game.cache.get_incident(id).action_card
            {
                let text = card
                    .civil_card
                    .description
                    .strip_prefix(
                        crate::content::incidents::great_persons::GREAT_PERSON_DESCRIPTION,
                    )
                    .unwrap_or(&card.civil_card.description)
                    .trim();
                description = format!(
                    "When played ({}): {text}",
                    if card.civil_card.action_type.free {
                        "free action"
                    } else {
                        "1 action"
                    }
                );
            }
            let mut available = p.resources.clone();
            for r in requests {
                let legacy_myths_label = r.cost.default.amount() == 1
                    && r.name.starts_with("You may pay 1 mood token for each city");
                let name = if legacy_myths_label {
                    "Pay 1 mood token to protect the one affected city, or pay nothing"
                } else {
                    &r.name
                };
                let field =
                    resource_field(&r.cost, name, r.optional, &available, false, &p.resources);
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
            description = match &h.origin {
                EventOrigin::CivilCard(33 | 34) if h.priority == 3 =>
                    "Choose the first of two advances from one category. Pay each research cost; event markers stay unchanged.",
                EventOrigin::CivilCard(33 | 34) =>
                    "Choose the second advance from the same category. Pay its research cost; event markers stay unchanged.",
                _ => match advance_mode(game, h) {
                    "paid" => "Choose an advance, then pay its research cost.",
                    "borrow" => "Use one advance until the end of your turn. No research bonuses or victory points.",
                    _ => "Choose one free advance.",
                },
            }.into();
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
                    let mut o = option(
                        id,
                        format!("{} #{} · {}", u.unit_type.name(game), id + 1, u.position),
                        "",
                        Some(u.position),
                    );
                    o["mapTarget"] =
                        json!({"kind":"unit","player":r.player,"unit":id,"unitType":u.unit_type});
                    o
                })
                .collect()
        }
        PersistentEventRequest::SelectCaptives(r) => {
            min = *r.needed.start();
            max = *r.needed.end();
            description.clone_from(&r.description);
            r.choices
                .iter()
                .map(|c| {
                    option(
                        c,
                        format!(
                            "{} · {}",
                            game.player(c.owner).civilization.name,
                            c.unit_type.name(game)
                        ),
                        "",
                        None,
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
                    let mut o = option(
                        s,
                        format!("{} · {}", structure_name(&s.structure), s.position),
                        "",
                        Some(s.position),
                    );
                    o["mapTarget"] = json!({"kind":"structure","structure":s.structure});
                    o
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
            description =
                great_seer_choice_description(game, h).unwrap_or_else(|| r.description.clone());
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
                    let mut choice = option(card, card.name(game), text, None);
                    choice["card"] = match card {
                        HandCard::ObjectiveCard(id) => {
                            let info = game.cache.get_objective_card(*id);
                            json!({"kind":"objective", "objectives":info.objectives.iter().map(|o| json!({
                                "name":o.name, "description":o.description,
                                "timing":match o.get_type() {
                                    ObjectiveType::Instant => "Instant",
                                    ObjectiveType::StatusPhase => "Status phase",
                                }
                            })).collect::<Vec<_>>()})
                        }
                        HandCard::ActionCard(id) => {
                            let info = game.cache.get_action_card(*id);
                            json!({"kind":"action", "name":info.civil_card.name,
                                "description":info.civil_card.description,
                                "tactics":info.tactics_card.as_ref().map(|t| json!({"name":t.name,"description":t.description}))})
                        }
                        HandCard::Wonder(_) => Value::Null,
                    };
                    choice
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
    let event_context = super::journal::decision_context(game, seat);
    if let Some(context) = &event_context {
        match context["piratePhase"].as_str() {
            Some("place") => {
                title = "Place pirates".into();
                description = format!(
                    "Ship {} of 2 · Choose a highlighted sea space.",
                    context["pirateShip"].as_u64().unwrap_or(1)
                );
            }
            Some("remove") => {
                title = "Return pirate ships".into();
                description =
                    format!("Return {min} to the supply so 2 pirate ships can be placed.");
            }
            Some("pay") => {
                title = "Pirate raid".into();
                description =
                    "Pay 1 resource or token total, regardless of city mood or number of pirates."
                        .into();
            }
            Some("mood") => {
                title = "Pirate raid".into();
                description = "Unable to pay · Lower the mood of one highlighted city.".into();
            }
            _ => {}
        }
    }
    Some(
        json!({"origin":h.origin,"name":title,"description":description,"min":min,"max":max,"options":options,"fields":fields,
        "eventContext":event_context,
        "reward":matches!(h.request,PersistentEventRequest::ResourceReward(_)),
        "tacticsSelection":tactics_selection,
        "advanceSelection":matches!(h.request,PersistentEventRequest::SelectAdvance(_)),
        "advanceMode":matches!(h.request,PersistentEventRequest::SelectAdvance(_)).then(|| advance_mode(game, h)),
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
            let mut available = game
                .player(game.current_event().player.index)
                .resources
                .clone();
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
        PersistentEventRequest::SelectCaptives(r) => {
            EventResponse::SelectCaptives(selected(values, r)?)
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

// Keep one attempt's context visible through action fees, range payment and rerolls.
pub(super) fn influence_context(game: &Game, seat: usize) -> Option<Value> {
    let event = game.events.last()?;
    if event.player.index != seat { return None; }
    let handler = game.current_event_handler()?;
    match &event.event_type {
        PersistentEventType::InfluenceCultureBoost(info) => {
            let stage = match &handler.request {
                PersistentEventRequest::BoolRequest(_) => "reroll",
                PersistentEventRequest::Payment(requests) if requests.iter().any(|r| r.optional) => "boost",
                PersistentEventRequest::Payment(_) => "range",
                _ => return None,
            };
            let a = &info.attempt;
            Some(json!({"stage":stage,"source":a.starting_city_position,"target":a.position,
                "name":if a.target_unit.is_some() { "Army unit".to_string() } else { structure_name(&a.structure) },
                "roll":(stage != "range").then_some(info.roll),"rollBonus":a.roll_boost,"threshold":crate::consts::INFLUENCE_MIN_ROLL}))
        }
        PersistentEventType::PayAction(payment) => {
            let crate::playing_actions::PlayingAction::InfluenceCultureAttempt(a) = &payment.action else {return None;};
            Some(json!({"stage":"payment","source":a.starting_position,"target":a.selected_structure.position,
                "name":if a.target_unit.is_some() {"Army unit".to_string()} else {structure_name(&a.selected_structure.structure)},
                "roll":null,"rollBonus":0,"threshold":crate::consts::INFLUENCE_MIN_ROLL}))
        }
        _ => None,
    }
}
