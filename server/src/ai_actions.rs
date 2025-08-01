use crate::action::{Action, ActionType};
use crate::advance::AdvanceAction;
use crate::ai_collect::{possible_collections, total_collect};
use crate::card::validate_card_selection;
use crate::city::{City, MoodState};
use crate::collect::{Collect, available_collect_actions, possible_resource_collections};
use crate::construct::{Construct, available_buildings, new_building_positions};
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::persistent_events::{
    EventResponse, HandCardsRequest, MultiRequest, PersistentEventRequest, PersistentEventState,
    PositionRequest, SelectedStructure, is_selected_structures_valid,
};
use crate::cultural_influence::{
    InfluenceCultureAttempt, available_influence_actions, available_influence_culture,
};
use crate::events::{EventOrigin, check_event_origin};
use crate::game::{Game, GameState};
use crate::happiness::{
    IncreaseHappiness, available_happiness_actions, happiness_city_restriction, happiness_cost,
};
use crate::payment::PaymentOptions;
use crate::player::{CostTrigger, Player};
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::recruit::{Recruit, recruit_cost};
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernment, government_advances};
use crate::unit::{UnitType, Units, validate_units_selection};
use crate::wonder::Wonder;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use std::vec;
//todo
//nicht nur maximale anzahl rekrutieren
//bewegung:
//Siedler: nur von stadt wegbewegen wo er gebaut wurde
//militär

struct PaymentCache {
    options: FxHashMap<PaymentOptions, FxHashMap<ResourcePile, Option<ResourcePile>>>,
}

impl PaymentCache {
    fn new() -> Self {
        PaymentCache {
            options: FxHashMap::default(),
        }
    }
}

pub struct AiActions {
    payment_cache: PaymentCache,
}

impl AiActions {
    #[must_use]
    pub fn new() -> Self {
        AiActions {
            payment_cache: PaymentCache::new(),
        }
    }

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
    /// # Panics
    ///
    /// Panics if the actions for any type is empty
    #[must_use]
    pub fn get_available_actions(&mut self, game: &Game) -> Vec<(ActionType, Vec<Action>)> {
        let actions = if let Some(event) = game.events.last() {
            vec![(
                ActionType::Response,
                responses(event, game.player(game.active_player()), game)
                    .into_iter()
                    .map(Action::Response)
                    .collect(),
            )]
        } else if game.state == GameState::Playing {
            base_actions(self, game)
        } else {
            vec![]
        };
        for (t, a) in &actions {
            assert!(
                !a.is_empty(),
                "Empty actions for action type: {t:?} - {:?}",
                game.events.last().as_ref().expect("event not found")
            );
        }
        actions
    }
}

impl Default for AiActions {
    fn default() -> Self {
        Self::new()
    }
}

type ActionFactory = (
    PlayingActionType,
    fn(&mut AiActions, &Player, &Game) -> Vec<Action>,
);

#[must_use]
fn base_actions(ai: &mut AiActions, game: &Game) -> Vec<(ActionType, Vec<Action>)> {
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

            let a = f(ai, p, game);
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
        if let Some(h) = calculate_increase_happiness(p, &prefer_custom_action(&happiness), game) {
            actions.push((
                ActionType::Playing(PlayingActionType::IncreaseHappiness),
                vec![Action::Playing(PlayingAction::IncreaseHappiness(h))],
            ));
        }
    }

    // InfluenceCultureAttempt,
    for action_type in available_influence_actions(game, p.index) {
        if let Some(i) = calculate_influence(game, p, &action_type) {
            actions.push((
                ActionType::Playing(PlayingActionType::Collect),
                vec![Action::Playing(PlayingAction::InfluenceCultureAttempt(
                    InfluenceCultureAttempt::new(i, action_type),
                ))],
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
            PlayingActionType::WonderCard(*card)
                .is_available(game, p.index)
                .is_ok()
                .then_some(Action::Playing(PlayingAction::WonderCard(*card)))
        })
        .collect_vec();

    if !wonder_cards.is_empty() {
        actions.push((
            ActionType::Playing(PlayingActionType::WonderCard(Wonder::Pyramids)),
            wonder_cards,
        ));
    }

    for info in game.available_custom_actions(p.index) {
        if info.action == CustomActionType::ImperialArmy {
            // buggy
            continue;
        }

        let cities = if info.city_bound().is_some() {
            p.cities
                .iter()
                .filter_map(|city| {
                    info.is_city_available(game, city)
                        .then_some(Some(city.position))
                })
                .collect_vec()
        } else {
            vec![None]
        };

        let a = info.action;
        for c in cities {
            actions.push((
                ActionType::Playing(PlayingActionType::Custom(a)),
                vec![Action::Playing(PlayingAction::Custom(CustomAction::new(
                    a, c,
                )))],
            ));
        }
    }

    actions
}

fn available_action_cards(game: &Game, p: &Player) -> Vec<Action> {
    p.action_cards
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
                // todo collect only is possible even if resources are wasted
                return None;
            }
            if *card == 7 || *card == 8 {
                // todo spy is slow
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
        .collect_vec()
}

fn payment(ai_actions: &mut AiActions, o: &PaymentOptions, p: &Player) -> ResourcePile {
    try_payment(ai_actions, o, p).expect("expected payment")
}

pub fn try_payment(
    ai_actions: &mut AiActions,
    o: &PaymentOptions,
    p: &Player,
) -> Option<ResourcePile> {
    let sum = o.default.amount();

    let mut max = p.resources.clone();
    for r in ResourceType::all() {
        let t = max.get_mut(&r);
        if *t > sum {
            *t = sum;
        }
    }

    if let Some(e) = ai_actions.payment_cache.options.get_mut(o) {
        // here we don't need to clone the payment options
        return e
            .entry(max)
            .or_insert_with_key(|available| o.first_valid_payment(available))
            .clone();
    }

    ai_actions
        .payment_cache
        .options
        .entry(o.clone())
        .or_default()
        .entry(max)
        .or_insert_with_key(|available| o.first_valid_payment(available))
        .clone()
}

fn advances(ai_actions: &mut AiActions, p: &Player, game: &Game) -> Vec<Action> {
    game.cache
        .get_advances()
        .values()
        .filter_map(|info| {
            let a = info.advance;
            if !p.can_advance_free(a, game) {
                return None;
            }
            try_payment(
                ai_actions,
                &p.advance_cost(a, game, CostTrigger::NoModifiers).cost,
                p,
            )
            .map(|r| Action::Playing(PlayingAction::Advance(AdvanceAction::new(a, r))))
        })
        .collect()
}

fn collect_actions(p: &Player, game: &Game) -> Vec<Action> {
    let collect = available_collect_actions(game, p.index);
    if collect.is_empty() {
        return vec![];
    }
    let action_type = prefer_custom_action(&collect);

    p.cities
        .iter()
        .filter(|city| city.can_activate())
        .flat_map(|city| {
            let info = possible_resource_collections(
                game,
                city.position,
                p.index,
                &check_event_origin(),
                CostTrigger::NoModifiers,
            );

            possible_collections(&info)
                .into_iter()
                .filter_map(|c| {
                    let total = total_collect(&c);
                    if !can_gain(p, total.clone()) {
                        return None;
                    }

                    Some(Action::Playing(PlayingAction::Collect(Collect::new(
                        city.position,
                        c,
                        action_type.clone(),
                    ))))
                })
                .collect_vec()
        })
        .collect_vec()
}

fn found_city(_ai_actions: &mut AiActions, p: &Player, game: &Game) -> Vec<Action> {
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

fn recruit(ai_actions: &mut AiActions, p: &Player, game: &Game) -> Vec<Action> {
    p.cities
        .iter()
        .flat_map(|city| {
            if city.can_activate() {
                recruit_actions(ai_actions, p, city, game)
            } else {
                vec![]
            }
        })
        .collect()
}

fn recruit_actions(
    ai_actions: &mut AiActions,
    player: &Player,
    city: &City,
    game: &Game,
) -> Vec<Action> {
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
                match recruit_cost(
                    game,
                    player,
                    &next,
                    city.position,
                    &[],
                    CostTrigger::NoModifiers,
                ) {
                    Ok(c) => {
                        cost = payment(ai_actions, &c.cost, player);
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
    game: &Game,
) -> Option<IncreaseHappiness> {
    // try to make the biggest cities happy - that's usually the best choice
    let mut all_steps: Vec<(Position, u8)> = vec![];
    let mut step_sum = 0;
    let mut cost = PaymentOptions::free();

    for c in player
        .cities
        .iter()
        .filter(|city| city.mood_state != MoodState::Happy)
        .sorted_by_key(|city| -(city.size() as i8))
    {
        if happiness_city_restriction(player, action_type).is_some_and(|r| r != c.position) {
            // city restriction is not met
            continue;
        }

        let steps = match c.mood_state {
            MoodState::Angry => 2,
            MoodState::Neutral => 1,
            MoodState::Happy => 0,
        };
        let new_steps_sum = step_sum + steps * c.size() as u8;

        let info = happiness_cost(
            player.index,
            new_steps_sum,
            CostTrigger::NoModifiers,
            action_type,
            game,
            &check_event_origin(),
        );
        if !info.cost.can_afford(&player.resources) {
            break;
        }
        all_steps.push((c.position, steps));
        step_sum = new_steps_sum;
        cost = info.cost;
    }

    (!all_steps.is_empty()).then_some(IncreaseHappiness::new(
        all_steps,
        cost.first_valid_payment(&player.resources)
            .expect("expected payment"),
        action_type.clone(),
    ))
}

fn prefer_custom_action(actions: &[PlayingActionType]) -> PlayingActionType {
    actions
        .iter()
        .find(|a| matches!(a, PlayingActionType::Custom(_)))
        .or_else(|| {
            actions
                .iter()
                .find(|a| !matches!(a, PlayingActionType::Custom(_)))
        })
        .cloned()
        .expect("expected at least one action type, either custom or base")
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
            vec![EventResponse::ResourceReward(
                r.reward.payment_options.default,
            )]
        }
        PersistentEventRequest::SelectAdvance(a) => a
            .choices
            .iter()
            .map(|c| EventResponse::SelectAdvance(*c))
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
            select_multi(&r.request, SelectMultiStrategy::All, |u| {
                validate_units_selection(u, game, player).is_ok()
            })
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
        PersistentEventRequest::ChangeGovernment => change_government(player, game),
        PersistentEventRequest::ExploreResolution => {
            vec![
                EventResponse::ExploreResolution(0),
                EventResponse::ExploreResolution(3),
            ]
        }
    }
}

fn change_government(p: &Player, game: &Game) -> Vec<EventResponse> {
    // change to the first available government and take the first advances
    let new = game
        .cache
        .get_governments()
        .iter()
        .find(|g| p.can_advance_ignore_contradicting(g.advances[0].advance, game))
        .expect("government not found");

    let advances = new
        .advances
        .iter()
        .dropping(1) // is taken implicitly
        .take(government_advances(p, game).len() - 1)
        .map(|a| a.advance)
        .collect_vec();

    vec![EventResponse::ChangeGovernmentType(ChangeGovernment::new(
        new.name.clone(),
        advances,
    ))]
}

fn hand_card_strategy(o: &EventOrigin, r: &HandCardsRequest) -> SelectMultiStrategy {
    match o {
        EventOrigin::Ability(n) if n == "Select Objective Cards to Complete" => {
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
        EventOrigin::Ability(n) if n == "Raze city" => SelectMultiStrategy::Min,
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

fn construct(ai_actions: &mut AiActions, p: &Player, game: &Game) -> Vec<Action> {
    p.cities
        .iter()
        .flat_map(|city| {
            if !city.can_activate() {
                return vec![];
            }
            get_construct_actions(ai_actions, game, p, city)
        })
        .collect()
}

pub(crate) fn get_construct_actions(
    ai_actions: &mut AiActions,
    game: &Game,
    p: &Player,
    city: &City,
) -> Vec<Action> {
    available_buildings(game, p.index, city.position, &[])
        .iter()
        .flat_map(|(building, cost)| {
            new_building_positions(game, *building, city)
                .iter()
                .map(|port| {
                    Action::Playing(PlayingAction::Construct(
                        Construct::new(
                            city.position,
                            *building,
                            payment(ai_actions, &cost.cost, p),
                        )
                        .with_port_position(*port),
                    ))
                })
                .collect_vec()
        })
        .collect()
}

pub(crate) fn can_gain_resource(player: &Player, r: ResourceType, amount: u8) -> bool {
    match r {
        ResourceType::MoodTokens | ResourceType::CultureTokens => true,
        _ => player.resources.get(&r) + amount <= player.resource_limit.get(&r),
    }
}

pub(crate) fn can_gain(player: &Player, r: ResourcePile) -> bool {
    r.into_iter().all(|(t, a)| can_gain_resource(player, t, a))
}
