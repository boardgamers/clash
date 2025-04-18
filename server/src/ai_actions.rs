use crate::action::{Action, ActionType};
use crate::card::validate_card_selection;
use crate::city::{City, MoodState};
use crate::collect::{available_collect_actions, collect_action};
use crate::construct::{Construct, available_buildings};
use crate::content::advances;
use crate::content::advances::economy::tax_options;
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::persistent_events::{
    ChangeGovernmentRequest, EventResponse, HandCardsRequest, MultiRequest, PersistentEventRequest,
    PersistentEventState, PositionRequest, SelectedStructure, is_selected_structures_valid,
};
use crate::cultural_influence::{
    available_influence_actions, available_influence_culture, influence_action,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::happiness::{available_happiness_actions, happiness_action, happiness_cost};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::playing_actions::{
    IncreaseHappiness, PlayingAction, PlayingActionType, Recruit, base_and_custom_action,
};
use crate::position::Position;
use crate::recruit::recruit_cost;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernment, ChangeGovernmentType, government_advances};
use crate::unit::{UnitType, Units};
use itertools::Itertools;
use std::vec;
//todo
//nicht nur maximale anzahl rekrutieren
//bewegung:
//Siedler: nur von stadt wegbewegen wo er gebaut wurde
//militär

///
/// Returns a list of available actions for the current player.
///
/// Some simplifications are made to help AI implementations:
/// - custom actions are preferred over basic actions if available.
/// - always pay default payment
/// - collect and select as much as possible (which is not always the best choice,
///   e.g. selecting to sacrifice a unit for an incident)
/// - move actions are not returned at all - this required special handling
///
#[must_use]
pub fn get_available_actions(game: &Game) -> Vec<(ActionType, Vec<Action>)> {
    if let Some(event) = game.events.last() {
        vec![(
            ActionType::Response,
            responses(event, game.player(game.active_player()), game)
                .into_iter()
                .map(Action::Response)
                .collect(),
        )]
    } else {
        base_actions(game)
    }
}

type ActionFactory = (PlayingActionType, fn(&Player, &Game) -> Vec<Action>);

#[must_use]
fn base_actions(game: &Game) -> Vec<(ActionType, Vec<Action>)> {
    let p = game.player(game.current_player_index);

    let factories: Vec<ActionFactory> = vec![
        (PlayingActionType::Advance, advances),
        (PlayingActionType::FoundCity, found_city),
        (PlayingActionType::Construct, construct),
        (PlayingActionType::Recruit, recruit),
    ];

    let mut actions: Vec<(ActionType, Vec<Action>)> = factories
        .iter()
        .filter_map(|(t, f)| {
            if t.is_available(game, p.index).is_err() {
                return None;
            }

            let a = f(p, game);
            (!a.is_empty()).then_some((ActionType::Playing(t.clone()), a))
        })
        .collect_vec();

    // MoveUnits -> special handling

    // Collect,
    let collect = collect_actions(p, game);
    if !collect.is_empty() {
        actions.push((ActionType::Playing(PlayingActionType::Collect), collect));
    }

    // IncreaseHappiness
    let happiness = available_happiness_actions(game, p.index);
    // if !happiness.is_empty() {
    // let action_type = prefer_custom_action(happiness); // todo custom action is buggy
    if let Some(action_type) = base_and_custom_action(happiness).0 {
        if let Some(h) = calculate_increase_happiness(p, &action_type) {
            actions.push((
                ActionType::Playing(PlayingActionType::IncreaseHappiness),
                vec![happiness_action(&action_type, h)],
            ));
        }
    }

    // InfluenceCultureAttempt,
    let influence = available_influence_actions(game, p.index);
    if !influence.is_empty() {
        let action_type = prefer_custom_action(influence);
        if let Some(i) = calculate_influence(game, p, &action_type) {
            actions.push((
                ActionType::Playing(PlayingActionType::Collect),
                vec![influence_action(&action_type, i)],
            ));
        }
    }

    // ActionCard,
    let action_cards = available_action_cards(game, p);

    if !action_cards.is_empty() {
        actions.push((
            ActionType::Playing(PlayingActionType::ActionCard(0)),
            action_cards,
        ));
    }

    // WonderCard,
    let wonder_cards = p
        .wonder_cards
        .iter()
        .filter_map(|card| {
            PlayingActionType::WonderCard(card.clone())
                .is_available(game, p.index)
                .is_ok()
                .then_some(Action::Playing(PlayingAction::WonderCard(card.clone())))
        })
        .collect_vec();

    if !wonder_cards.is_empty() {
        actions.push((
            ActionType::Playing(PlayingActionType::WonderCard(String::new())),
            wonder_cards,
        ));
    }

    for (a, _) in game.available_custom_actions(p.index) {
        let option = match a {
            CustomActionType::Sports | CustomActionType::Theaters | // todo
            CustomActionType::ArtsInfluenceCultureAttempt
            | CustomActionType::VotingIncreaseHappiness
            | CustomActionType::FreeEconomyCollect => None, // handled above
            CustomActionType::AbsolutePower => Some(CustomAction::AbsolutePower),
            CustomActionType::ForcedLabor => Some(CustomAction::ForcedLabor),
            CustomActionType::CivilLiberties => Some(CustomAction::CivilLiberties),
            CustomActionType::Taxes => try_payment(&tax_options(p), p).map(CustomAction::Taxes),
        };

        if let Some(action) = option {
            actions.push((
                ActionType::Playing(PlayingActionType::Custom(a.clone().info())),
                vec![Action::Playing(PlayingAction::Custom(action))],
            ));
        }
    }

    actions
}

fn available_action_cards(game: &Game, p: &Player) -> Vec<Action> {
    let action_cards = p
        .action_cards
        .iter()
        .filter_map(|card| {
            if *card == 126 || *card == 17 || *card == 18 {
                // todo construct only is buggy
                return None;
            }
            if *card == 120 {
                // todo great prophet is buggy
                return None;
            }
            if *card == 124 {
                // todo great warlord needs movement to work
                return None;
            }
            if *card == 19 || *card == 20 || *card == 29 || *card == 30 {
                // todo collect only is buggy
                return None;
            }
            if *card == 15 || *card == 16 {
                // todo influence only is buggy
                return None;
            }
            if *card == 33 || *card == 34 {
                // todo synergies is slow
                return None;
            }

            PlayingActionType::ActionCard(*card)
                .is_available(game, p.index)
                .is_ok()
                .then_some(Action::Playing(PlayingAction::ActionCard(*card)))
        })
        .collect_vec();
    action_cards
}

fn payment(o: &PaymentOptions, p: &Player) -> ResourcePile {
    o.first_valid_payment(&p.resources)
        .expect("expected payment")
}

fn payment_with_action(
    o: &PaymentOptions,
    p: &Player,
    playing_action_type: &PlayingActionType,
) -> ResourcePile {
    o.first_valid_payment(&playing_action_type.remaining_resources(p))
        .expect("expected payment")
}

fn try_payment(o: &PaymentOptions, p: &Player) -> Option<ResourcePile> {
    o.first_valid_payment(&p.resources)
}

fn advances(p: &Player, _game: &Game) -> Vec<Action> {
    advances::get_all()
        .iter()
        .filter_map(|a| {
            if deny_advance(&a.name) {
                return None;
            }

            if !p.can_advance_free(a) {
                return None;
            }
            try_payment(&p.advance_cost(a, None).cost, p).map(|r| {
                Action::Playing(PlayingAction::Advance {
                    advance: a.name.clone(),
                    payment: r,
                })
            })
        })
        .collect()
}

fn deny_advance(name: &str) -> bool {
    //todo collect cache doesn't work, because husbandry can only be used once per turn
    //correct cache: 1) only store total in cache 2) sort by distance 3) add husbandry flag

    name == "Husbandry"
}

fn collect_actions(p: &Player, game: &Game) -> Vec<Action> {
    let collect = available_collect_actions(game, p.index);
    if collect.is_empty() {
        return vec![];
    }
    let action_type = prefer_custom_action(collect);

    p.cities
        .iter()
        .filter(|city| city.can_activate())
        .flat_map(|city| {
            city.possible_collections.iter().filter_map({
                let value = action_type.clone();
                move |c| {
                    p.can_gain(c.total.clone())
                        .then_some(collect_action(&value, c.clone()))
                }
            })
        })
        .collect_vec()
}

fn found_city(p: &Player, game: &Game) -> Vec<Action> {
    p.units
        .iter()
        .filter_map(|u| {
            u.can_found_city(game)
                .then_some(Action::Playing(PlayingAction::FoundCity { settler: u.id }))
        })
        .collect()
}

fn recruit_strategies() -> Vec<Vec<UnitType>> {
    vec![
        vec![UnitType::Ship],
        vec![UnitType::Elephant, UnitType::Cavalry, UnitType::Infantry],
        vec![UnitType::Infantry], // in case we can't build cavalry and elephant
        vec![UnitType::Settler],
        vec![UnitType::Settler, UnitType::Infantry], // guarded
    ]
}

fn recruit(p: &Player, _game: &Game) -> Vec<Action> {
    p.cities
        .iter()
        .flat_map(|city| {
            if city.can_activate() {
                recruit_actions(p, city)
            } else {
                vec![]
            }
        })
        .collect()
}

fn recruit_actions(player: &Player, city: &City) -> Vec<Action> {
    recruit_strategies()
        .iter()
        .map(|strategy| {
            let mut units = Units::empty();
            let mut cost = ResourcePile::empty();
            let mut i = 0;
            loop {
                // cycle through the strategy - adding units, checking if still possible
                // after each step
                let unit_type = strategy[i];
                i = (i + 1) % strategy.len();

                let mut next = units.clone();
                next += &unit_type;
                match recruit_cost(player, &next, city.position, None, &[], None) {
                    Ok(c) => {
                        cost = payment(&c.cost, player);
                        units = next;
                    }
                    Err(_) => {
                        // not possible to recruit this unit
                        break;
                    }
                }
            }
            (units, cost)
        })
        .filter(|(units, _cost)| units.amount() > 0)
        .unique()
        .map(|(units, cost)| {
            Action::Playing(PlayingAction::Recruit(Recruit::new(
                &units,
                city.position,
                cost,
            )))
        })
        .collect()
}

fn calculate_increase_happiness(
    player: &Player,
    action_type: &PlayingActionType,
) -> Option<IncreaseHappiness> {
    // try to make the biggest cities happy - that's usually the best choice
    let mut all_steps: Vec<(Position, u32)> = vec![];
    let mut cost = PaymentOptions::free();
    let available = action_type.remaining_resources(player);

    for c in player
        .cities
        .iter()
        .filter(|city| city.mood_state != MoodState::Happy)
        .sorted_by_key(|city| -(city.size() as i8))
    {
        let steps = match c.mood_state {
            MoodState::Angry => 2,
            MoodState::Neutral => 1,
            MoodState::Happy => 0,
        };
        let mut new_steps = all_steps.clone();
        new_steps.push((c.position, steps));

        let info = happiness_cost(player, &new_steps, None);
        if !info.cost.can_afford(&available) {
            break;
        }
        all_steps = new_steps;
        cost = info.cost;
    }

    (!all_steps.is_empty()).then_some(IncreaseHappiness::new(
        all_steps,
        payment_with_action(&cost, player, action_type),
    ))
}

fn prefer_custom_action(actions: Vec<PlayingActionType>) -> PlayingActionType {
    let (action, custom) = base_and_custom_action(actions);
    action.unwrap_or_else(|| {
        PlayingActionType::Custom(custom.expect("custom action should be present").info())
    })
}

#[allow(clippy::match_same_arms)]
#[must_use]
fn responses(event: &PersistentEventState, player: &Player, game: &Game) -> Vec<EventResponse> {
    let h = event.player.handler.as_ref().expect("handler");
    match h.request.clone() {
        PersistentEventRequest::Payment(p) => {
            let mut available = player.resources.clone();
            vec![EventResponse::Payment(
                p.into_iter()
                    .map(|p| {
                        let o = &p.cost;
                        let pile = o
                            .first_valid_payment(&available)
                            .unwrap_or(ResourcePile::empty());
                        available -= pile.clone();
                        pile
                    })
                    .collect(),
            )]
        }
        PersistentEventRequest::ResourceReward(r) => {
            vec![EventResponse::ResourceReward(r.reward.default)]
        }
        PersistentEventRequest::SelectAdvance(a) => a
            .choices
            .iter()
            .filter(|c| !deny_advance(c.as_str()))
            .map(|c| EventResponse::SelectAdvance(c.clone()))
            .collect(),
        PersistentEventRequest::SelectPlayer(p) => p
            .choices
            .iter()
            .map(|c| EventResponse::SelectPlayer(*c))
            .collect(),
        PersistentEventRequest::SelectPositions(p) => {
            select_multi(&p, select_position_strategy(&h.origin, &p), |_| true)
                .into_iter()
                .map(EventResponse::SelectPositions)
                .collect()
        }
        PersistentEventRequest::SelectUnitType(t) => t
            .choices
            .iter()
            .map(|c| EventResponse::SelectUnitType(*c))
            .collect(),
        PersistentEventRequest::SelectUnits(r) => {
            select_multi(&r.request, SelectMultiStrategy::All, |_| true)
                .into_iter()
                .map(EventResponse::SelectUnits)
                .collect()
        }
        PersistentEventRequest::SelectStructures(r) => {
            select_multi(&r, SelectMultiStrategy::All, |s| {
                is_selected_structures_valid(game, s)
            })
            .into_iter()
            .map(EventResponse::SelectStructures)
            .collect()
        }
        PersistentEventRequest::SelectHandCards(r) => {
            select_multi(&r, hand_card_strategy(&h.origin, &r), |v| {
                validate_card_selection(v, game).is_ok()
            })
            .into_iter()
            .map(EventResponse::SelectHandCards)
            .collect()
        }
        PersistentEventRequest::BoolRequest(_) => {
            vec![EventResponse::Bool(false), EventResponse::Bool(true)]
        }
        PersistentEventRequest::ChangeGovernment(c) => change_government(player, &c),
        PersistentEventRequest::ExploreResolution => {
            vec![
                EventResponse::ExploreResolution(0),
                EventResponse::ExploreResolution(3),
            ]
        }
    }
}

fn change_government(p: &Player, c: &ChangeGovernmentRequest) -> Vec<EventResponse> {
    if c.optional {
        vec![EventResponse::ChangeGovernmentType(
            ChangeGovernmentType::KeepGovernment,
        )]
    } else {
        // change to the first available government and take the first advances
        let new = advances::get_governments()
            .iter()
            .find(|g| p.can_advance_in_change_government(&g.advances[0]))
            .expect("government not found");

        let advances = new
            .advances
            .iter()
            .dropping(1) // is taken implicitly
            .take(government_advances(p).len() - 1)
            .map(|a| a.name.clone())
            .collect_vec();

        vec![EventResponse::ChangeGovernmentType(
            ChangeGovernmentType::ChangeGovernment(ChangeGovernment::new(
                new.name.clone(),
                advances,
            )),
        )]
    }
}

fn hand_card_strategy(o: &EventOrigin, r: &HandCardsRequest) -> SelectMultiStrategy {
    match o {
        EventOrigin::Builtin(n) if n == "Select Objective Cards to Complete" => {
            SelectMultiStrategy::Max
        }
        EventOrigin::CivilCard(_)
            if r.description == "Select a Wonder, Action, or Objective card to swap" =>
        {
            SelectMultiStrategy::Min // powerset takes too long
        }
        _ => SelectMultiStrategy::All,
    }
}

fn select_position_strategy(o: &EventOrigin, _r: &PositionRequest) -> SelectMultiStrategy {
    match o {
        EventOrigin::Builtin(n) if n == "Raze city" => SelectMultiStrategy::Min,
        _ => SelectMultiStrategy::All,
    }
}

#[must_use]
#[derive(Clone, Debug, Copy)]
enum SelectMultiStrategy {
    Min,
    Max,
    All,
}

fn select_multi<T: Clone>(
    r: &MultiRequest<T>,
    strategy: SelectMultiStrategy,
    validator: impl Fn(&[T]) -> bool,
) -> Vec<Vec<T>> {
    let mut filter = r
        .choices
        .clone()
        .into_iter()
        .powerset()
        .filter(|p| validator(p) && r.needed.contains(&(p.len() as u8)));

    match strategy {
        SelectMultiStrategy::Min => filter.next().map_or(Vec::new(), |v| vec![v]),
        SelectMultiStrategy::All => filter.collect(),
        SelectMultiStrategy::Max => filter.last().map_or(Vec::new(), |v| vec![v]),
    }
}

#[must_use]
fn calculate_influence(
    game: &Game,
    player: &Player,
    action_type: &PlayingActionType,
) -> Option<SelectedStructure> {
    available_influence_culture(game, player.index, action_type)
        .into_iter()
        .filter_map(|(s, info)| info.ok().map(|i| (s, i.roll_boost, i.prevent_boost)))
        .sorted_by_key(|(_, roll, prevent)| roll + u8::from(*prevent) / 2)
        .next()
        .map(|(s, _, _)| s)
}

fn construct(p: &Player, game: &Game) -> Vec<Action> {
    p.cities
        .iter()
        .flat_map(|city| {
            if !city.can_activate() {
                return vec![];
            }
            get_construct_actions(game, p, city)
        })
        .collect()
}

pub(crate) fn get_construct_actions(game: &Game, p: &Player, city: &City) -> Vec<Action> {
    available_buildings(game, p.index, city.position)
        .iter()
        .map(|(building, cost, port)| {
            Action::Playing(PlayingAction::Construct(
                Construct::new(city.position, *building, payment(&cost.cost, p))
                    .with_port_position(*port),
            ))
        })
        .collect()
}
