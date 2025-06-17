use crate::action_cost::{ActionCostOncePerTurn, ActionCostOncePerTurnBuilder};
use crate::advance::Advance;
use crate::card::{HandCard, validate_card_selection_for_origin};
use crate::city::City;
use crate::combat::{Combat, update_combat_strength};
use crate::combat_listeners::CombatStrength;
use crate::content::ability::{Ability, AbilityBuilder};
use crate::content::custom_actions::{
    CustomActionActionExecution, CustomActionExecution, CustomActionInfo,
};
use crate::content::persistent_events::{
    AdvanceRequest, EventResponse, HandCardsRequest, MultiRequest, PaymentRequest,
    PersistentEventHandler, PersistentEventRequest, PlayerRequest, PositionRequest,
    ResourceRewardRequest, SelectedStructure, StructuresRequest, UnitTypeRequest, UnitsRequest,
};
use crate::events::{Event, EventOrigin, EventPlayer};
use crate::player::Player;
use crate::player_events::{PersistentEvent, PersistentEvents, TransientEvents};
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::resource::pay_cost;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, validate_units_selection_for_origin};
use crate::{content::custom_actions::CustomActionType, game::Game, player_events::PlayerEvents};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::RangeInclusive;
use std::sync::Arc;

pub(crate) type AbilityInitializer = Arc<dyn Fn(&mut Game, usize) + Sync + Send>;

pub(crate) type AbilityInitializerWithPrioDelta = Arc<dyn Fn(&mut Game, usize, i32) + Sync + Send>;

pub(crate) struct SelectedChoice<C> {
    pub player_index: usize,
    pub player_name: String,
    pub origin: EventOrigin,
    pub actively_selected: bool,
    pub choice: C,
}

impl<C> SelectedChoice<C> {
    pub fn new(p: &EventPlayer, actively_selected: bool, choice: C) -> Self {
        Self {
            player_index: p.index,
            player_name: p.name.clone(),
            origin: p.origin.clone(),
            actively_selected,
            choice,
        }
    }

    pub fn player(&self) -> EventPlayer {
        EventPlayer::new(
            self.player_index,
            self.player_name.clone(),
            self.origin.clone(),
        )
    }

    pub fn other_player(&self, player_index: usize, game: &Game) -> EventPlayer {
        EventPlayer::new(
            player_index,
            game.player_name(player_index),
            self.origin.clone(),
        )
    }

    pub fn log(&self, game: &mut Game, message: &str) {
        game.log_with_origin(self.player_index, &self.origin, message);
    }
}

impl Display for SelectedChoice<usize> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.player_name)
    }
}

#[derive(Clone)]
pub struct AbilityListeners {
    initializer: AbilityInitializerWithPrioDelta,
    deinitializer: AbilityInitializer,
    once_initializer: AbilityInitializer,
    once_deinitializer: AbilityInitializer,
}

impl AbilityListeners {
    pub fn init(&self, game: &mut Game, player_index: usize) {
        self.init_with_prio_delta(game, player_index, 0);
    }

    pub fn init_with_prio_delta(&self, game: &mut Game, player_index: usize, prio_delta: i32) {
        (self.initializer)(game, player_index, prio_delta);
    }

    pub fn deinit(&self, game: &mut Game, player_index: usize) {
        (self.deinitializer)(game, player_index);
    }

    pub fn once_init(&self, game: &mut Game, player_index: usize) {
        self.init(game, player_index);
        (self.once_initializer)(game, player_index);
    }

    pub fn once_deinit(&self, game: &mut Game, player_index: usize) {
        self.deinit(game, player_index);
        (self.once_deinitializer)(game, player_index);
    }
}

pub(crate) struct AbilityInitializerBuilder {
    initializers: Vec<AbilityInitializerWithPrioDelta>,
    deinitializers: Vec<AbilityInitializer>,
    once_initializers: Vec<AbilityInitializer>,
    once_deinitializers: Vec<AbilityInitializer>,
}

impl AbilityInitializerBuilder {
    pub fn new() -> Self {
        Self {
            initializers: Vec::new(),
            deinitializers: Vec::new(),
            once_initializers: Vec::new(),
            once_deinitializers: Vec::new(),
        }
    }

    pub(crate) fn add_initializer<F>(&mut self, initializer: F)
    where
        F: Fn(&mut Game, usize, i32) + 'static + Sync + Send,
    {
        self.initializers.push(Arc::new(initializer));
    }

    pub(crate) fn add_deinitializer<F>(&mut self, deinitializer: F)
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.deinitializers.push(Arc::new(deinitializer));
    }

    pub(crate) fn add_once_initializer<F>(&mut self, initializer: F)
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.once_initializers.push(Arc::new(initializer));
    }

    pub(crate) fn add_once_deinitializer<F>(&mut self, deinitializer: F)
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.once_deinitializers.push(Arc::new(deinitializer));
    }

    pub(crate) fn build(self) -> AbilityListeners {
        AbilityListeners {
            initializer: join_ability_initializers_with_prio_delta(self.initializers),
            deinitializer: join_ability_initializers(self.deinitializers),
            once_initializer: join_ability_initializers(self.once_initializers),
            once_deinitializer: join_ability_initializers(self.once_deinitializers),
        }
    }
}

#[must_use]
pub(crate) trait AbilityInitializerSetup: Sized {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder;

    fn get_key(&self) -> EventOrigin;

    fn name(&self) -> String;

    fn description(&self) -> String;

    fn add_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize, i32) + 'static + Sync + Send,
    {
        self.builder().add_initializer(initializer);
        self
    }

    fn add_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.builder().add_deinitializer(deinitializer);
        self
    }

    fn add_once_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.builder().add_once_initializer(initializer);
        self
    }

    fn add_once_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static + Sync + Send,
    {
        self.builder().add_once_deinitializer(deinitializer);
        self
    }

    fn add_transient_event_listener<T, U, V, E, F>(
        self,
        event: E,
        priority: i32,
        listener: F,
    ) -> Self
    where
        T: Clone + PartialEq,
        E: Fn(&mut TransientEvents) -> &mut Event<T, U, V> + 'static + Clone + Sync + Send,
        F: Fn(&mut T, &U, &V, &EventPlayer) + 'static + Clone + Sync + Send,
    {
        add_player_event_listener(
            self,
            move |events| event(&mut events.transient),
            priority,
            move |value, u, v, (), p| listener(value, u, v, p),
        )
    }

    fn add_combat_strength_listener(
        self,
        priority: i32,
        listener: impl Fn(&Game, &Combat, &mut CombatStrength, CombatRole)
        + Clone
        + 'static
        + Sync
        + Send,
    ) -> Self {
        self.add_simple_persistent_event_listener(
            |event| &mut event.combat_round_start,
            priority,
            move |game, p, s| {
                update_combat_strength(game, p.index, s, {
                    let l = listener.clone();
                    move |game, combat, s, role| l(game, combat, s, role)
                });
            },
        )
    }

    fn add_persistent_event_listener<E, V>(
        self,
        event: E,
        priority: i32,
        start_custom_phase: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<PersistentEventRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        end_custom_phase: impl Fn(
            &mut Game,
            &EventPlayer,
            EventResponse,
            PersistentEventRequest,
            &mut V,
        )
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        add_player_event_listener(
            self,
            move |e| event(&mut e.persistent),
            priority,
            move |game, _i, (), details, p| {
                if let Some(mut phase) = game.events.pop() {
                    if let Some(ref c) = phase.player.handler {
                        if let Some(ref action) = c.response {
                            if c.priority == priority {
                                let mut current = phase.clone();
                                current
                                    .player
                                    .handler
                                    .as_mut()
                                    .expect("current missing")
                                    .response = None;
                                let r = c.request.clone();
                                let a = action.clone();
                                phase.player.handler = None;
                                game.events.push(phase);
                                end_custom_phase.clone()(game, p, a, r, details);
                                return;
                            }
                        }
                    }
                    let is_current = phase.player.handler.is_some();
                    game.events.push(phase);
                    if is_current {
                        return;
                    }
                }

                if game
                    .current_event()
                    .player
                    .last_priority_used
                    .is_some_and(|last| last <= priority)
                {
                    // already handled before
                    return;
                }

                // need to set the priority here, because the event might be
                // trigger another event
                game.current_event_mut().player.last_priority_used = Some(priority);

                if let Some(request) = start_custom_phase(game, p, details) {
                    game.current_event_mut().player.handler = Some(PersistentEventHandler {
                        priority,
                        request: request.clone(),
                        response: None,
                        origin: p.origin.clone(),
                    });
                }
            },
        )
    }

    fn add_simple_persistent_event_listener<V, E, F>(
        self,
        event: E,
        priority: i32,
        listener: F,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        F: Fn(&mut Game, &EventPlayer, &mut V) + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, p, details| {
                // only for the listener
                listener(game, p, details);
                None
            },
            |_, _, _, _, _| {},
        )
    }

    fn add_payment_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<Vec<PaymentRequest>>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<ResourcePile>>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, p, details| {
                request(game, p, details)
                    .filter(|r| r.iter().any(|r| game.player(p.index).can_afford(&r.cost)))
                    .map(PersistentEventRequest::Payment)
            },
            move |game, p, action, request, details| {
                if let PersistentEventRequest::Payment(requests) = &request {
                    if let EventResponse::Payment(payments) = action {
                        assert_eq!(requests.len(), payments.len());
                        for (request, payment) in requests.iter().zip(payments.iter()) {
                            pay_cost(game, p.index, request, payment);
                        }
                        gain_reward(game, &SelectedChoice::new(p, true, payments), details);
                        return;
                    }
                }
                panic!("Expected payment response");
            },
        )
    }

    fn add_resource_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<ResourceRewardRequest>
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_resource_request_with_response(event, priority, request, move |_game, _s, _| {})
    }

    fn add_resource_request_with_response<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<ResourceRewardRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        response: impl Fn(&mut Game, &SelectedChoice<ResourcePile>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        let response2 = response.clone();
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, player, details| {
                let req = request(game, player, details);
                if let Some(r) = &req {
                    if r.reward.payment_options.possible_resource_types().len() == 1 {
                        let pile = r.reward.payment_options.default_payment();
                        response2(
                            game,
                            &r.reward.selected_choice(player, pile.clone(), false),
                            details,
                        );
                        player.gain_resources(game, pile);
                        return None;
                    }
                }
                req.map(PersistentEventRequest::ResourceReward)
            },
            move |game, player, action, request, details| {
                if let PersistentEventRequest::ResourceReward(request) = &request {
                    if let EventResponse::ResourceReward(reward) = action {
                        assert!(
                            request.reward.payment_options.is_valid_payment(&reward),
                            "Invalid reward"
                        );
                        response(
                            game,
                            &request.reward.selected_choice(player, reward.clone(), true),
                            details,
                        );
                        player.gain_resources(game, reward);
                        return;
                    }
                }
                panic!("Expected resource reward response");
            },
        )
    }

    fn add_bool_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<String>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<bool>, &mut V) + 'static + Clone + Sync + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, p, details| {
                request(game, p, details).map(PersistentEventRequest::BoolRequest)
            },
            move |game, p, action, request, details| {
                if let PersistentEventRequest::BoolRequest(_) = &request {
                    if let EventResponse::Bool(reward) = action {
                        gain_reward(game, &SelectedChoice::new(p, true, reward), details);
                        return;
                    }
                }
                panic!("Boolean request expected");
            },
        )
    }

    fn add_advance_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<AdvanceRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Advance>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, Advance, AdvanceRequest, V>(
            event,
            priority,
            |r| &r.choices,
            PersistentEventRequest::SelectAdvance,
            |request, action| {
                if let PersistentEventRequest::SelectAdvance(request) = &request {
                    if let EventResponse::SelectAdvance(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Advance request expected");
            },
            request,
            gain_reward,
        )
    }

    fn add_position_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<PositionRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<Position>>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, Position, PositionRequest, V>(
            event,
            priority,
            |r| r,
            PersistentEventRequest::SelectPositions,
            |request, action| {
                if let PersistentEventRequest::SelectPositions(request) = &request {
                    if let EventResponse::SelectPositions(reward) = action {
                        return (request.choices.clone(), reward, request.needed.clone());
                    }
                }
                panic!("Position request expected");
            },
            request,
            gain_reward,
        )
    }

    fn add_player_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<PlayerRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<usize>, &mut V) + 'static + Clone + Sync + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, usize, PlayerRequest, V>(
            event,
            priority,
            |r| &r.choices,
            PersistentEventRequest::SelectPlayer,
            |request, action| {
                if let PersistentEventRequest::SelectPlayer(request) = &request {
                    if let EventResponse::SelectPlayer(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Player request expected");
            },
            request,
            gain_reward,
        )
    }

    fn add_unit_type_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<UnitTypeRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<UnitType>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, UnitType, UnitTypeRequest, V>(
            event,
            priority,
            |r| &r.choices,
            PersistentEventRequest::SelectUnitType,
            |request, action| {
                if let PersistentEventRequest::SelectUnitType(request) = &request {
                    if let EventResponse::SelectUnitType(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Unit type request expected");
            },
            request,
            gain_reward,
        )
    }

    fn add_hand_card_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<HandCardsRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        cards_selected: impl Fn(&mut Game, &SelectedChoice<Vec<HandCard>>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, HandCard, HandCardsRequest, V>(
            event,
            priority,
            |r| r,
            PersistentEventRequest::SelectHandCards,
            |request, action| {
                if let PersistentEventRequest::SelectHandCards(request) = &request {
                    if let EventResponse::SelectHandCards(choices) = action {
                        return (request.choices.clone(), choices, request.needed.clone());
                    }
                }
                panic!("Hand Cards request expected");
            },
            request,
            move |game, c, details| {
                validate_card_selection_for_origin(&c.choice, game, &c.origin)
                    .expect("Invalid card selection - this should not happen");
                cards_selected(game, c, details);
            },
        )
    }

    fn add_units_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<UnitsRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        units_selected: impl Fn(&mut Game, &SelectedChoice<Vec<u32>>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, u32, UnitsRequest, V>(
            event,
            priority,
            |r| &r.request,
            PersistentEventRequest::SelectUnits,
            |request, action| {
                if let PersistentEventRequest::SelectUnits(request) = &request {
                    if let EventResponse::SelectUnits(choices) = action {
                        return (
                            request.request.choices.clone(),
                            choices,
                            request.request.needed.clone(),
                        );
                    }
                }
                panic!("Units request expected");
            },
            request,
            move |game, s, details| {
                validate_units_selection_for_origin(
                    &s.choice,
                    game.player(s.player_index),
                    &s.origin,
                )
                .expect("Invalid units selection - this should not happen");
                units_selected(game, s, details);
            },
        )
    }

    fn add_structures_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<StructuresRequest>
        + 'static
        + Clone
        + Sync
        + Send,
        structures_selected: impl Fn(&mut Game, &SelectedChoice<Vec<SelectedStructure>>, &mut V)
        + 'static
        + Clone
        + Sync
        + Send,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, SelectedStructure, StructuresRequest, V>(
            event,
            priority,
            |r| r,
            PersistentEventRequest::SelectStructures,
            |request, action| {
                if let PersistentEventRequest::SelectStructures(request) = &request {
                    if let EventResponse::SelectStructures(choices) = action {
                        return (request.choices.clone(), choices, request.needed.clone());
                    }
                }
                panic!("Structures request expected");
            },
            request,
            structures_selected,
        )
    }

    fn add_choice_reward_request_listener<E, C, R, V>(
        self,
        event: E,
        priority: i32,
        get_choices: impl Fn(&R) -> &Vec<C> + 'static + Clone + Sync + Send,
        to_request: impl Fn(R) -> PersistentEventRequest + 'static + Clone + Sync + Send,
        from_request: impl Fn(&PersistentEventRequest, EventResponse) -> (Vec<C>, C)
        + 'static
        + Clone
        + Sync
        + Send,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<R> + 'static + Clone + Sync + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<C>, &mut V) + 'static + Clone + Sync + Send,
    ) -> Self
    where
        C: Clone + PartialEq + Debug,
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        let g = gain_reward.clone();
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, p, details| {
                if let Some(r) = request(game, p, details) {
                    let choices = get_choices(&r);
                    if choices.is_empty() {
                        return None;
                    }
                    if choices.len() == 1 {
                        g(
                            game,
                            &SelectedChoice::new(p, false, choices[0].clone()),
                            details,
                        );
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, p, action, request, details| {
                let (choices, selected) = from_request(&request, action);
                assert!(
                    choices.contains(&selected),
                    "Invalid choice {selected:?} - available: {choices:?}"
                );
                gain_reward(game, &SelectedChoice::new(p, true, selected), details);
            },
        )
    }

    fn add_multi_choice_reward_request_listener<E, C, R, V>(
        self,
        event: E,
        priority: i32,
        get_request: impl Fn(&R) -> &MultiRequest<C> + 'static + Clone + Sync + Send,
        to_request: impl Fn(R) -> PersistentEventRequest + 'static + Clone + Sync + Send,
        from_request: impl Fn(
            &PersistentEventRequest,
            EventResponse,
        ) -> (Vec<C>, Vec<C>, RangeInclusive<u8>)
        + 'static
        + Clone
        + Sync
        + Send,
        request: impl Fn(&mut Game, &EventPlayer, &mut V) -> Option<R> + 'static + Clone + Sync + Send,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<C>>, &mut V) + 'static + Clone + Sync + Send,
    ) -> Self
    where
        C: Clone + PartialEq + Debug,
        E: Fn(&mut PersistentEvents) -> &mut PersistentEvent<V> + 'static + Clone + Sync + Send,
        V: Clone + PartialEq,
    {
        let g = gain_reward.clone();
        self.add_persistent_event_listener(
            event,
            priority,
            move |game, p, details| {
                if let Some(r) = request(game, p, details) {
                    let m = get_request(&r);
                    if m.choices.is_empty() || m.needed.clone().max() == Some(0) {
                        return None;
                    }
                    if Some(m.choices.len() as u8) == m.needed.clone().min()
                        && m.needed.clone().min() == m.needed.clone().max()
                    {
                        g(
                            game,
                            &SelectedChoice::new(p, false, m.choices.clone()),
                            details,
                        );
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, p, action, request, details| {
                let (choices, selected, needed) = from_request(&request, action);
                assert!(
                    selected.iter().all(|s| choices.contains(s)),
                    "Invalid choice {selected:?} - available: {choices:?}",
                );
                assert!(
                    needed.contains(&(selected.len() as u8)),
                    "Invalid choice count: {} (min: {}, max: {})",
                    selected.len(),
                    needed.start(),
                    needed.end(),
                );
                gain_reward(game, &SelectedChoice::new(p, true, selected), details);
            },
        )
    }

    fn add_custom_action(
        self,
        action: CustomActionType,
        cost: impl Fn(ActionCostOncePerTurnBuilder) -> ActionCostOncePerTurn + Send + Sync + 'static,
        ability: impl Fn(AbilityBuilder) -> AbilityBuilder + Sync + Send + 'static,
        can_play: impl Fn(&Game, &Player) -> bool + Sync + Send + 'static,
    ) -> Self {
        let name = self.name();
        let desc = self.description();
        self.add_custom_action_execution(
            action,
            cost,
            CustomActionExecution::Action(CustomActionActionExecution::new(
                ability(Ability::builder(&name, &desc)).build(),
                Arc::new(can_play),
                None,
            )),
        )
    }

    fn add_custom_action_with_city_checker(
        self,
        action: CustomActionType,
        cost: impl Fn(ActionCostOncePerTurnBuilder) -> ActionCostOncePerTurn + Send + Sync + 'static,
        ability: impl Fn(AbilityBuilder) -> AbilityBuilder + Sync + Send + 'static,
        can_play: impl Fn(&Game, &Player) -> bool + Sync + Send + 'static,
        city_checker: impl Fn(&Game, &City) -> bool + Sync + Send + 'static,
    ) -> Self {
        let name = self.name();
        let desc = self.description();
        self.add_custom_action_execution(
            action,
            cost,
            CustomActionExecution::Action(CustomActionActionExecution::new(
                ability(Ability::builder(&name, &desc)).build(),
                Arc::new(can_play),
                Some(Arc::new(city_checker)),
            )),
        )
    }

    fn add_action_modifier(
        self,
        action: CustomActionType,
        info: impl Fn(ActionCostOncePerTurnBuilder) -> ActionCostOncePerTurn + Send + Sync + 'static,
        base_action: PlayingActionType,
    ) -> Self {
        self.add_custom_action_execution(action, info, CustomActionExecution::Modifier(base_action))
    }

    fn add_custom_action_execution(
        self,
        action: CustomActionType,
        cost: impl Fn(ActionCostOncePerTurnBuilder) -> ActionCostOncePerTurn + Send + Sync + 'static,
        execution: CustomActionExecution,
    ) -> Self {
        let deinitializer_action = action;
        let key = self.get_key().clone();
        let exec = execution.clone();
        self.add_initializer(move |game, player_index, _prio_delta| {
            game.player_mut(player_index).custom_actions.insert(
                action,
                CustomActionInfo::new(
                    action,
                    exec.clone(),
                    key.clone(),
                    cost(ActionCostOncePerTurnBuilder::new(action)),
                ),
            );
        })
        .add_deinitializer(move |game, player_index| {
            game.player_mut(player_index)
                .custom_actions
                .remove(&deinitializer_action);
        })
    }
}

fn join_ability_initializers(setup: Vec<AbilityInitializer>) -> AbilityInitializer {
    Arc::new(move |game: &mut Game, player_index: usize| {
        for initializer in &setup {
            initializer(game, player_index);
        }
    })
}

fn join_ability_initializers_with_prio_delta(
    setup: Vec<AbilityInitializerWithPrioDelta>,
) -> AbilityInitializerWithPrioDelta {
    Arc::new(
        move |game: &mut Game, player_index: usize, prio_delta: i32| {
            for initializer in &setup {
                initializer(game, player_index, prio_delta);
            }
        },
    )
}

fn add_player_event_listener<S, T, U, V, W, E, F>(
    setup: S,
    event: E,
    priority: i32,
    listener: F,
) -> S
where
    S: AbilityInitializerSetup,
    T: Clone + PartialEq,
    W: Clone + PartialEq,
    E: Fn(&mut PlayerEvents) -> &mut Event<T, U, V, W> + 'static + Clone + Sync + Send,
    F: Fn(&mut T, &U, &V, &mut W, &EventPlayer) + 'static + Clone + Sync + Send,
{
    let key = setup.get_key().clone();
    let deinitialize_event = event.clone();
    let initializer = move |game: &mut Game, player_index: usize, prio_delta: i32| {
        let player_name = game.player_name(player_index);
        let e = event(&mut game.players[player_index].events);
        e.inner
            .as_mut()
            .unwrap_or_else(|| panic!("event {} should be set: {key:?}", e.name))
            .add_listener_mut(
                listener.clone(),
                priority + prio_delta,
                EventPlayer::new(player_index, player_name, key.clone()),
            );
    };
    let key = setup.get_key().clone();
    let deinitializer = move |game: &mut Game, player_index: usize| {
        let e = deinitialize_event(&mut game.players[player_index].events);
        e.inner
            .as_mut()
            .unwrap_or_else(|| panic!("event {} should be set: {key:?}", e.name))
            .remove_listener_mut_by_key(&key);
    };
    setup
        .add_initializer(initializer)
        .add_deinitializer(deinitializer)
}

#[allow(clippy::map_entry)]
pub(crate) fn once_per_turn_advance<F, T, U, V>(
    id: Advance,
    value: &mut T,
    u: &U,
    v: &V,
    get_info: impl Fn(&mut T) -> &mut HashMap<String, String> + Clone + 'static + Sync + Send,
    listener: F,
) where
    F: Fn(&mut T, &U, &V) + 'static + Clone + Sync + Send,
{
    let key = id.id();
    if !get_info(value).contains_key(&key) {
        listener(value, u, v);
        get_info(value).insert(key, "used".to_string());
    }
}
