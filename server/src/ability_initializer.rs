use crate::card::HandCard;
use crate::combat::{update_combat_strength, Combat};
use crate::combat_listeners::CombatStrength;
use crate::content::custom_phase_actions::{
    AdvanceRequest, CurrentEventHandler, CurrentEventRequest, EventResponse, HandCardsRequest,
    MultiRequest, PaymentRequest, PlayerRequest, PositionRequest, ResourceRewardRequest,
    SelectedStructure, StructuresRequest, UnitTypeRequest, UnitsRequest,
};
use crate::events::{Event, EventOrigin};
use crate::player_events::{CurrentEvent, PersistentEvents, TransientEvents};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::CombatRole;
use crate::unit::UnitType;
use crate::{content::custom_actions::CustomActionType, game::Game, player_events::PlayerEvents};
use std::collections::HashMap;
use std::ops::RangeInclusive;

pub(crate) type AbilityInitializer = Box<dyn Fn(&mut Game, usize)>;

pub struct SelectedChoice<C> {
    pub player_index: usize,
    pub player_name: String,
    pub actively_selected: bool,
    pub choice: C,
}

impl<C> SelectedChoice<C> {
    pub fn new(player_index: usize, player_name: &str, actively_selected: bool, choice: C) -> Self {
        Self {
            player_index,
            player_name: player_name.to_string(),
            actively_selected,
            choice,
        }
    }
}

pub struct AbilityListeners {
    pub initializer: AbilityInitializer,
    pub deinitializer: AbilityInitializer,
    pub one_time_initializer: AbilityInitializer,
    pub undo_deinitializer: AbilityInitializer,
}

pub(crate) struct AbilityInitializerBuilder {
    initializers: Vec<AbilityInitializer>,
    deinitializers: Vec<AbilityInitializer>,
    one_time_initializers: Vec<AbilityInitializer>,
    undo_deinitializers: Vec<AbilityInitializer>,
}

impl AbilityInitializerBuilder {
    pub fn new() -> Self {
        Self {
            initializers: Vec::new(),
            deinitializers: Vec::new(),
            one_time_initializers: Vec::new(),
            undo_deinitializers: Vec::new(),
        }
    }

    pub(crate) fn add_ability_initializer<F>(&mut self, initializer: F)
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.initializers.push(Box::new(initializer));
    }

    pub(crate) fn add_ability_deinitializer<F>(&mut self, deinitializer: F)
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.deinitializers.push(Box::new(deinitializer));
    }

    pub(crate) fn add_one_time_ability_initializer<F>(&mut self, initializer: F)
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.one_time_initializers.push(Box::new(initializer));
    }

    pub(crate) fn add_ability_undo_deinitializer<F>(&mut self, deinitializer: F)
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.undo_deinitializers.push(Box::new(deinitializer));
    }

    pub(crate) fn build(self) -> AbilityListeners {
        AbilityListeners {
            initializer: join_ability_initializers(self.initializers),
            deinitializer: join_ability_initializers(self.deinitializers),
            one_time_initializer: join_ability_initializers(self.one_time_initializers),
            undo_deinitializer: join_ability_initializers(self.undo_deinitializers),
        }
    }
}

pub(crate) trait AbilityInitializerSetup: Sized {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder;

    fn get_key(&self) -> EventOrigin;

    fn add_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.builder().add_ability_initializer(initializer);
        self
    }

    fn add_ability_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.builder().add_ability_deinitializer(deinitializer);
        self
    }

    fn add_one_time_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.builder().add_one_time_ability_initializer(initializer);
        self
    }

    fn add_ability_undo_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.builder().add_ability_undo_deinitializer(deinitializer);
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
        E: Fn(&mut TransientEvents) -> &mut Event<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        add_player_event_listener(
            self,
            move |events| event(&mut events.transient),
            priority,
            move |value, u, v, ()| listener(value, u, v),
        )
    }

    fn add_combat_round_start_listener(
        self,
        priority: i32,
        listener: impl Fn(&Game, &Combat, &mut CombatStrength, CombatRole) + Clone + 'static,
    ) -> Self {
        self.add_simple_persistent_event_listener(
            |event| &mut event.on_combat_round_start,
            priority,
            move |game, p, _, s| {
                update_combat_strength(game, p, s, {
                    let l = listener.clone();
                    move |game, combat, s, role| l(game, combat, s, role)
                });
            },
        )
    }

    fn add_once_per_turn_listener<T, U, V, E, F>(
        self,
        event: E,
        get_info: impl Fn(&mut T) -> &mut HashMap<String, String> + 'static + Clone,
        listener: F,
        priority: i32,
    ) -> Self
    where
        T: Clone + PartialEq,
        E: Fn(&mut TransientEvents) -> &mut Event<T, U, V, ()> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let id = self.get_key().id();
        self.add_transient_event_listener(event, priority, move |value, u, v| {
            if !get_info(value).contains_key(&id) {
                listener(value, u, v);
                get_info(value).insert(id.clone(), "used".to_string());
            }
        })
    }

    fn add_current_event_listener<E, V>(
        self,
        event: E,
        priority: i32,
        start_custom_phase: impl Fn(&mut Game, usize, &str, &mut V) -> Option<CurrentEventRequest>
            + 'static
            + Clone,
        end_custom_phase: impl Fn(&mut Game, usize, &str, EventResponse, CurrentEventRequest, &mut V)
            + 'static
            + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        let origin = self.get_key();
        add_player_event_listener(
            self,
            move |e| event(&mut e.persistent),
            priority,
            move |game, i, (), details| {
                let player_index = i.player;
                let player_name = game.player_name(player_index);

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
                                end_custom_phase.clone()(
                                    game,
                                    player_index,
                                    &player_name,
                                    a,
                                    r,
                                    details,
                                );
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
                    .is_some_and(|last| last < priority)
                {
                    // already handled before
                    return;
                }

                if let Some(request) = start_custom_phase(game, player_index, &player_name, details)
                {
                    let s = game.current_event_mut();
                    s.player.last_priority_used = Some(priority);
                    s.player.handler = Some(CurrentEventHandler {
                        priority,
                        request: request.clone(),
                        response: None,
                        origin: origin.clone(),
                    });
                };
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
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        F: Fn(&mut Game, usize, &str, &mut V) + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, player_name, details| {
                // only for the listener
                listener(game, player_index, player_name, details);
                None
            },
            |_, _, _, _, _, _| {},
        )
    }

    fn add_payment_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut V) -> Option<Vec<PaymentRequest>> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<ResourcePile>>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                request(game, player_index, details)
                    .filter(|r| {
                        r.iter()
                            .any(|r| game.get_player(player_index).can_afford(&r.cost))
                    })
                    .map(CurrentEventRequest::Payment)
            },
            move |game, player_index, player_name, action, request, details| {
                if let CurrentEventRequest::Payment(requests) = &request {
                    if let EventResponse::Payment(payments) = action {
                        assert_eq!(requests.len(), payments.len());
                        for (request, payment) in requests.iter().zip(payments.iter()) {
                            let zero_payment = payment.is_empty() && request.optional;
                            if !zero_payment {
                                game.players[player_index].pay_cost(&request.cost, payment);
                            }
                        }
                        gain_reward(
                            game,
                            &SelectedChoice::new(player_index, player_name, true, payments),
                            details,
                        );
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<ResourceRewardRequest> + 'static + Clone,
        gain_reward_log: impl Fn(&Game, &SelectedChoice<ResourcePile>, &mut V) -> Vec<String>
            + 'static
            + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        let g = gain_reward_log.clone();
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                let req = request(game, player_index, details);
                if let Some(r) = &req {
                    if r.reward.possible_resource_types().len() == 1 {
                        let player_name = game.player_name(player_index);
                        let r = r.reward.default_payment();
                        for log in g(
                            game,
                            &SelectedChoice::new(player_index, &player_name, false, r.clone()),
                            details,
                        ) {
                            game.add_info_log_item(&log);
                        }
                        game.players[player_index].gain_resources(r);
                        return None;
                    }
                }
                req.map(CurrentEventRequest::ResourceReward)
            },
            move |game, player_index, player_name, action, request, details| {
                if let CurrentEventRequest::ResourceReward(request) = &request {
                    if let EventResponse::ResourceReward(reward) = action {
                        assert!(request.reward.is_valid_payment(&reward), "Invalid reward");
                        for log in &gain_reward_log(
                            game,
                            &SelectedChoice::new(player_index, player_name, true, reward.clone()),
                            details,
                        ) {
                            game.add_info_log_item(log);
                        }
                        game.players[player_index].gain_resources(reward);
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<String> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<bool>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                request(game, player_index, details).map(CurrentEventRequest::BoolRequest)
            },
            move |game, player_index, player_name, action, request, details| {
                if let CurrentEventRequest::BoolRequest(_) = &request {
                    if let EventResponse::Bool(reward) = action {
                        gain_reward(
                            game,
                            &SelectedChoice::new(player_index, player_name, true, reward),
                            details,
                        );
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<AdvanceRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<String>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, String, AdvanceRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CurrentEventRequest::SelectAdvance,
            |request, action| {
                if let CurrentEventRequest::SelectAdvance(request) = &request {
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<PositionRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<Position>>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, Position, PositionRequest, V>(
            event,
            priority,
            |r| r,
            CurrentEventRequest::SelectPositions,
            |request, action| {
                if let CurrentEventRequest::SelectPositions(request) = &request {
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<PlayerRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<usize>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, usize, PlayerRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CurrentEventRequest::SelectPlayer,
            |request, action| {
                if let CurrentEventRequest::SelectPlayer(request) = &request {
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<UnitTypeRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<UnitType>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_choice_reward_request_listener::<E, UnitType, UnitTypeRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CurrentEventRequest::SelectUnitType,
            |request, action| {
                if let CurrentEventRequest::SelectUnitType(request) = &request {
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
        request: impl Fn(&mut Game, usize, &mut V) -> Option<HandCardsRequest> + 'static + Clone,
        cards_selected: impl Fn(&mut Game, &SelectedChoice<Vec<HandCard>>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, HandCard, HandCardsRequest, V>(
            event,
            priority,
            |r| r,
            CurrentEventRequest::SelectHandCards,
            |request, action| {
                if let CurrentEventRequest::SelectHandCards(request) = &request {
                    if let EventResponse::SelectHandCards(choices) = action {
                        return (request.choices.clone(), choices, request.needed.clone());
                    }
                }
                panic!("Hand Cards request expected");
            },
            request,
            cards_selected,
        )
    }

    fn add_units_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut V) -> Option<UnitsRequest> + 'static + Clone,
        units_selected: impl Fn(&mut Game, &SelectedChoice<Vec<u32>>, &mut V) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, u32, UnitsRequest, V>(
            event,
            priority,
            |r| &r.request,
            CurrentEventRequest::SelectUnits,
            |request, action| {
                if let CurrentEventRequest::SelectUnits(request) = &request {
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
            move |game, c, details| {
                units_selected(game, c, details);
            },
        )
    }

    fn add_structures_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &mut V) -> Option<StructuresRequest> + 'static + Clone,
        structures_selected: impl Fn(&mut Game, &SelectedChoice<Vec<SelectedStructure>>, &mut V)
            + 'static
            + Clone,
    ) -> Self
    where
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        self.add_multi_choice_reward_request_listener::<E, SelectedStructure, StructuresRequest, V>(
            event,
            priority,
            |r| r,
            CurrentEventRequest::SelectStructures,
            |request, action| {
                if let CurrentEventRequest::SelectStructures(request) = &request {
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
        get_choices: impl Fn(&R) -> &Vec<C> + 'static + Clone,
        to_request: impl Fn(R) -> CurrentEventRequest + 'static + Clone,
        from_request: impl Fn(&CurrentEventRequest, EventResponse) -> (Vec<C>, C) + 'static + Clone,
        request: impl Fn(&mut Game, usize, &mut V) -> Option<R> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<C>, &mut V) + 'static + Clone,
    ) -> Self
    where
        C: Clone + PartialEq,
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        let g = gain_reward.clone();
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, player_name, details| {
                if let Some(r) = request(game, player_index, details) {
                    let choices = get_choices(&r);
                    if choices.is_empty() {
                        return None;
                    }
                    if choices.len() == 1 {
                        g(
                            game,
                            &SelectedChoice::new(
                                player_index,
                                player_name,
                                false,
                                choices[0].clone(),
                            ),
                            details,
                        );
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, player_index, player_name, action, request, details| {
                let (choices, selected) = from_request(&request, action);
                assert!(choices.contains(&selected), "Invalid choice");
                gain_reward(
                    game,
                    &SelectedChoice::new(player_index, player_name, true, selected),
                    details,
                );
            },
        )
    }

    fn add_multi_choice_reward_request_listener<E, C, R, V>(
        self,
        event: E,
        priority: i32,
        get_request: impl Fn(&R) -> &MultiRequest<C> + 'static + Clone,
        to_request: impl Fn(R) -> CurrentEventRequest + 'static + Clone,
        from_request: impl Fn(&CurrentEventRequest, EventResponse) -> (Vec<C>, Vec<C>, RangeInclusive<u8>)
            + 'static
            + Clone,
        request: impl Fn(&mut Game, usize, &mut V) -> Option<R> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, &SelectedChoice<Vec<C>>, &mut V) + 'static + Clone,
    ) -> Self
    where
        C: Clone + PartialEq,
        E: Fn(&mut PersistentEvents) -> &mut CurrentEvent<V> + 'static + Clone,
        V: Clone + PartialEq,
    {
        let g = gain_reward.clone();
        self.add_current_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                if let Some(r) = request(game, player_index, details) {
                    let m = get_request(&r);
                    if m.choices.is_empty() || m.needed.clone().max() == Some(0) {
                        return None;
                    }
                    if Some(m.choices.len() as u8) == m.needed.clone().min()
                        && m.needed.clone().min() == m.needed.clone().max()
                    {
                        g(
                            game,
                            &SelectedChoice::new(
                                player_index,
                                &game.player_name(player_index),
                                false,
                                m.choices.clone(),
                            ),
                            details,
                        );
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, player_index, player_name, action, request, details| {
                let (choices, selected, needed) = from_request(&request, action);
                assert!(
                    selected.iter().all(|s| choices.contains(s)),
                    "Invalid choice"
                );
                assert!(
                    needed.contains(&(selected.len() as u8)),
                    "Invalid choice count"
                );
                gain_reward(
                    game,
                    &SelectedChoice::new(player_index, player_name, true, selected),
                    details,
                );
            },
        )
    }

    fn add_custom_action(self, action: CustomActionType) -> Self {
        let deinitializer_action = action.clone();
        let key = self.get_key().clone();
        self.add_ability_initializer(move |game, player_index| {
            let player = &mut game.players[player_index];
            player.custom_actions.insert(action.clone(), key.clone());
        })
        .add_ability_deinitializer(move |game, player_index| {
            let player = &mut game.players[player_index];
            player.custom_actions.remove(&deinitializer_action);
        })
    }
}

fn join_ability_initializers(setup: Vec<AbilityInitializer>) -> AbilityInitializer {
    Box::new(move |game: &mut Game, player_index: usize| {
        for initializer in &setup {
            initializer(game, player_index);
        }
    })
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
    E: Fn(&mut PlayerEvents) -> &mut Event<T, U, V, W> + 'static + Clone,
    F: Fn(&mut T, &U, &V, &mut W) + 'static + Clone,
{
    let key = setup.get_key().clone();
    let deinitialize_event = event.clone();
    let initializer = move |game: &mut Game, player_index: usize| {
        event(&mut game.players[player_index].events)
            .inner
            .as_mut()
            .expect("events should be set")
            .add_listener_mut(listener.clone(), priority, key.clone());
    };
    let key = setup.get_key().clone();
    let deinitializer = move |game: &mut Game, player_index: usize| {
        deinitialize_event(&mut game.players[player_index].events)
            .inner
            .as_mut()
            .expect("events should be set")
            .remove_listener_mut_by_key(&key);
    };
    setup
        .add_ability_initializer(initializer)
        .add_ability_deinitializer(deinitializer)
}
