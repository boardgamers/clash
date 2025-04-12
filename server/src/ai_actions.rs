use crate::action::{Action, ActionType};
use crate::card::validate_card_selection;
use crate::city::{City, MoodState};
use crate::collect::{
    CollectInfo, PositionCollection, add_collect, available_collect_actions, collect_action,
    get_total_collection, possible_resource_collections,
};
use crate::construct::{Construct, available_buildings};
use crate::content::advances;
use crate::content::advances::economy::tax_options;
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::persistent_events::{
    EventResponse, HandCardsRequest, MultiRequest, PersistentEventRequest, PersistentEventState,
    PositionRequest, SelectedStructure, is_selected_structures_valid,
};
use crate::cultural_influence::{
    available_influence_actions, available_influence_culture, influence_action,
};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::happiness::{available_happiness_actions, happiness_action, increase_happiness_cost};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::playing_actions::{
    Collect, IncreaseHappiness, PlayingAction, PlayingActionType, Recruit, base_and_custom_action,
};
use crate::position::Position;
use crate::recruit::recruit_cost;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::status_phase::ChangeGovernmentType;
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
    if !happiness.is_empty() {
        let action_type = prefer_custom_action(happiness);

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
        if let Some(i) = calculate_influence(game, p) {
            actions.push((
                ActionType::Playing(PlayingActionType::Collect),
                vec![influence_action(&action_type, i)],
            ));
        }
    }

    // ActionCard,
    let action_cards = p
        .action_cards
        .iter()
        .filter_map(|card| {
            PlayingActionType::ActionCard(*card)
                .is_available(game, p.index)
                .is_ok()
                .then_some(Action::Playing(PlayingAction::ActionCard(*card)))
        })
        .collect_vec();

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

fn payment(o: &PaymentOptions, p: &Player) -> ResourcePile {
    o.first_valid_payment(&p.resources)
        .expect("expected payment")
}

fn try_payment(o: &PaymentOptions, p: &Player) -> Option<ResourcePile> {
    o.first_valid_payment(&p.resources)
}

fn advances(p: &Player, _game: &Game) -> Vec<Action> {
    advances::get_all()
        .iter()
        .filter_map(|a| {
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
            city_collections(game, p, city)
                .into_iter()
                .map(|c| collect_action(&action_type, c))
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
        .filter(|(units, _cost)| units.sum() > 0)
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

fn calculate_increase_happiness(player: &Player, action_type: &PlayingActionType) -> Option<IncreaseHappiness> {
    // try to make the biggest cities happy - that's usually the best choice
    let mut cities = vec![];
    let mut cost = PaymentOptions::resources(action_type.cost().cost);

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

        let mut new_city_cost =
            increase_happiness_cost(player, c, steps).expect("cost should be available");
        new_city_cost.cost.default += cost.default.clone();
        if try_payment(&new_city_cost.cost, player).is_none() {
            break;
        }
        cost = new_city_cost.cost;
        cities.push((c.position, steps));
    }

    (!cities.is_empty()).then_some(IncreaseHappiness::new(cities, payment(&cost, player)))
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
            vec![EventResponse::Payment(
                p.into_iter().map(|p| payment(&p.cost, player)).collect(),
            )]
        }
        PersistentEventRequest::ResourceReward(r) => {
            vec![EventResponse::ResourceReward(r.reward.default)]
        }
        PersistentEventRequest::SelectAdvance(a) => a
            .choices
            .iter()
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
        PersistentEventRequest::ChangeGovernment(_c) => {
            vec![EventResponse::ChangeGovernmentType(
                ChangeGovernmentType::KeepGovernment,
            )]
        }
        PersistentEventRequest::ExploreResolution => {
            vec![
                EventResponse::ExploreResolution(0),
                EventResponse::ExploreResolution(3),
            ]
        }
    }
}

fn hand_card_strategy(o: &EventOrigin, _r: &HandCardsRequest) -> SelectMultiStrategy {
    match o {
        EventOrigin::Builtin(n) if n == "Select Objective Cards to Complete" => {
            SelectMultiStrategy::Max
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
pub fn city_collections(game: &Game, player: &Player, city: &City) -> Vec<Collect> {
    let info = possible_resource_collections(game, city.position, player.index, &[], &[]);

    let all = ResourceType::all()
        .into_iter()
        .filter(|r| {
            info.choices
                .iter()
                .any(|(_, choices)| choices.iter().any(|pile| pile.get(r) > 0))
                && can_gain_resource(player, *r)
        })
        .collect_vec();
    let l = all.len();
    all.into_iter()
        .permutations(l)
        .map(|priority| city_collection(game, player, city, &priority))
        .unique_by(Collect::total)
        .filter(|c| c.total().amount() > 0) // todo check earlier?
        .collect_vec()
}

fn can_gain_resource(p: &Player, r: ResourceType) -> bool {
    match r {
        ResourceType::MoodTokens | ResourceType::CultureTokens => true,
        _ => p.resources.get(&r) < p.resource_limit.get(&r),
    }
}

fn city_collection(
    game: &Game,
    player: &Player,
    city: &City,
    priority: &[ResourceType],
) -> Collect {
    let mut c: Vec<PositionCollection> = vec![];

    loop {
        let info = possible_resource_collections(game, city.position, player.index, &c, &c);

        let Some((pos, pile)) = pick_resource(player, &info, &c, priority) else {
            break;
        };
        let new = add_collect(&info, pos, &pile, &c);

        if get_total_collection(game, player.index, city.position, &new).is_err() {
            break;
        }

        c = new;
    }

    Collect::new(city.position, c)
}

fn pick_resource(
    player: &Player,
    info: &CollectInfo,
    collected: &[PositionCollection],
    priority: &[ResourceType],
) -> Option<(Position, ResourcePile)> {
    let used = collected
        .iter()
        .chunk_by(|c| c.position)
        .into_iter()
        .map(|(p, group)| (p, group.map(|c| c.times).sum::<u32>()))
        .collect_vec();

    let available = info
        .choices
        .iter()
        // .sorted_by_key(|(pos, _)| **pos)
        .filter(|(pos, _)| {
            let u = used
                .iter()
                .find_map(|(p, u)| (*p == **pos).then_some(*u))
                .unwrap_or(0);

            u < info.max_per_tile
        })
        .collect_vec();

    priority.iter().find_map(|r| {
        if !can_gain_resource(player, *r) {
            return None;
        }

        available.iter().find_map(|(pos, choices)| {
            choices
                .iter()
                .find_map(|pile| (pile.get(r) > 0).then_some((**pos, pile.clone())))
        })
    })
}

#[must_use]
fn calculate_influence(game: &Game, player: &Player) -> Option<SelectedStructure> {
    available_influence_culture(game, player.index)
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
            if city.can_activate() {
                return vec![];
            }
            get_construct_actions(game, p, city)
        })
        .collect()
}

pub(crate) fn get_construct_actions(game: &Game, p: &Player, city: &City) -> Vec<Action> {
    available_buildings(game, p.index, city.position)
        .iter()
        .map(|(building, port)| {
            Action::Playing(PlayingAction::Construct(
                Construct::new(
                    city.position,
                    *building,
                    payment(&p.construct_cost(game, *building, None).cost, p),
                )
                .with_port_position(*port),
            ))
        })
        .collect()
}
