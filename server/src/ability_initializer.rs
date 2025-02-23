use crate::content::custom_phase_actions::{
    AdvanceRewardRequest, CurrentCustomPhaseEvent, CustomPhaseEventAction, CustomPhaseRequest,
    CustomPhaseUnitsRequest, PaymentRequest, PositionRequest, ResourceRewardRequest,
    UnitTypeRequest,
};
use crate::events::{Event, EventOrigin};
use crate::game::UndoContext;
use crate::player_events::{CustomPhaseEvent, PlayerCommands};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::{content::custom_actions::CustomActionType, game::Game, player_events::PlayerEvents};
use std::collections::HashMap;

pub(crate) type AbilityInitializer = Box<dyn Fn(&mut Game, usize)>;

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

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        T: Clone + PartialEq,
        E: Fn(&mut PlayerEvents) -> &mut Event<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.get_key().clone();
        let deinitialize_event = event.clone();
        let initializer = move |game: &mut Game, player_index: usize| {
            event(&mut game.players[player_index].events)
                .inner
                .as_mut()
                .expect("events should be set")
                .add_listener_mut(listener.clone(), priority, key.clone());
        };
        let key = self.get_key().clone();
        let deinitializer = move |game: &mut Game, player_index: usize| {
            deinitialize_event(&mut game.players[player_index].events)
                .inner
                .as_mut()
                .expect("events should be set")
                .remove_listener_mut_by_key(&key);
        };
        self.add_ability_initializer(initializer)
            .add_ability_deinitializer(deinitializer)
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
        E: Fn(&mut PlayerEvents) -> &mut Event<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let id = self.get_key().id();
        self.add_player_event_listener(
            event,
            move |value, u, v| {
                if !get_info(value).contains_key(&id) {
                    listener(value, u, v);
                    get_info(value).insert(id.clone(), "used".to_string());
                }
            },
            priority,
        )
    }

    fn add_state_change_event_listener<E, V>(
        self,
        event: E,
        priority: i32,
        start_custom_phase: impl Fn(&mut Game, usize, &str, &V) -> Option<CustomPhaseRequest>
            + 'static
            + Clone, //return option<custom phase state>
        end_custom_phase: impl Fn(&mut Game, usize, &str, CustomPhaseEventAction, CustomPhaseRequest)
            + 'static
            + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        let origin = self.get_key();
        self.add_player_event_listener(
            event,
            move |game, i, details| {
                let player_index = i.player;
                let player_name = game.players[player_index].get_name();

                if let Some(mut phase) = game.custom_phase_state.pop() {
                    if let Some(ref c) = phase.current {
                        if let Some(ref action) = c.response {
                            if c.priority == priority {
                                let mut ctx = phase.clone();
                                ctx.current.as_mut().expect("current missing").response = None;
                                game.undo_context_stack
                                    .push(UndoContext::CustomPhaseEvent(ctx));
                                let r = c.request.clone();
                                let a = action.clone();
                                phase.current = None;
                                game.custom_phase_state.push(phase);
                                end_custom_phase.clone()(game, player_index, &player_name, a, r);
                                return;
                            }
                        }
                    }
                    let is_current = phase.current.is_some();
                    game.custom_phase_state.push(phase);
                    if is_current {
                        return;
                    }
                }

                if game
                    .current_custom_phase()
                    .last_priority_used
                    .is_some_and(|last| last < priority)
                {
                    // already handled before
                    return;
                }

                if let Some(request) = start_custom_phase(game, player_index, &player_name, details)
                {
                    let s = game.current_custom_phase_mut();
                    s.last_priority_used = Some(priority);
                    s.current = Some(CurrentCustomPhaseEvent {
                        priority,
                        player_index,
                        request: request.clone(),
                        response: None,
                        origin: origin.clone(),
                    });
                };
            },
            priority,
        )
    }

    fn add_payment_request_with_commands_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<Vec<PaymentRequest>> + 'static + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &Vec<ResourcePile>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_payment_request_listener(
            event,
            priority,
            request,
            move |game, player_index, _player_name, payments| {
                game.with_commands(player_index, |commands, game| {
                    gain_reward(commands, game, payments);
                });
            },
        )
    }

    fn add_payment_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<Vec<PaymentRequest>> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &Vec<ResourcePile>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                request(game, player_index, details).map(CustomPhaseRequest::Payment)
            },
            move |game, player_index, player_name, action, request| {
                if let CustomPhaseRequest::Payment(requests) = &request {
                    if let CustomPhaseEventAction::Payment(payments) = action {
                        assert_eq!(requests.len(), payments.len());
                        for (request, payment) in requests.iter().zip(payments.iter()) {
                            let zero_payment = payment.is_empty() && request.optional;
                            if !zero_payment {
                                game.players[player_index].pay_cost(&request.cost, payment);
                            }
                        }
                        gain_reward(game, player_index, player_name, &payments);
                        return;
                    }
                }
                panic!("Invalid state");
            },
        )
    }

    fn add_resource_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<ResourceRewardRequest> + 'static + Clone,
        gain_reward_log: impl Fn(&Game, usize, &str, &ResourcePile, bool) -> String + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        let g = gain_reward_log.clone();
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                let req = request(game, player_index, details);
                if let Some(r) = &req {
                    if r.reward.possible_resource_types().len() == 1 {
                        let player_name = game.players[player_index].get_name();
                        let r = r.reward.default_payment();
                        game.add_info_log_item(&g(game, player_index, &player_name, &r, false));
                        game.players[player_index].gain_resources(r);
                        return None;
                    }
                }
                req.map(CustomPhaseRequest::ResourceReward)
            },
            move |game, player_index, player_name, action, request| {
                if let CustomPhaseRequest::ResourceReward(request) = &request {
                    if let CustomPhaseEventAction::ResourceReward(reward) = action {
                        assert!(request.reward.is_valid_payment(&reward), "Invalid payment");
                        game.add_info_log_item(&gain_reward_log(
                            game,
                            player_index,
                            player_name,
                            &reward,
                            true,
                        ));
                        game.players[player_index].gain_resources(reward);
                        return;
                    }
                }
                panic!("Invalid state");
            },
        )
    }

    fn add_bool_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> bool + 'static + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, bool) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                request(game, player_index, details).then_some(CustomPhaseRequest::BoolRequest)
            },
            move |game, player_index, player_name, action, request| {
                if let CustomPhaseRequest::BoolRequest = &request {
                    if let CustomPhaseEventAction::Bool(reward) = action {
                        gain_reward(game, player_index, player_name, reward);
                        return;
                    }
                }
                panic!("Invalid state");
            },
        )
    }

    fn add_advance_reward_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<AdvanceRewardRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &String, bool) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_choice_reward_request_listener::<E, String, AdvanceRewardRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CustomPhaseRequest::AdvanceReward,
            |request, action| {
                if let CustomPhaseRequest::AdvanceReward(request) = &request {
                    if let CustomPhaseEventAction::AdvanceReward(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Invalid state");
            },
            request,
            gain_reward,
        )
    }

    fn add_position_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<PositionRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &Position) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_choice_reward_request_listener::<E, Position, PositionRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CustomPhaseRequest::SelectPosition,
            |request, action| {
                if let CustomPhaseRequest::SelectPosition(request) = &request {
                    if let CustomPhaseEventAction::SelectPosition(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Invalid state");
            },
            request,
            move |game, player_index, _player_name, choice, _selected| {
                game.with_commands(player_index, |commands, game| {
                    gain_reward(commands, game, choice);
                });
            },
        )
    }

    fn add_unit_type_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<UnitTypeRequest> + 'static + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &UnitType) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_choice_reward_request_listener::<E, UnitType, UnitTypeRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CustomPhaseRequest::SelectUnitType,
            |request, action| {
                if let CustomPhaseRequest::SelectUnitType(request) = &request {
                    if let CustomPhaseEventAction::SelectUnitType(reward) = action {
                        return (request.choices.clone(), reward);
                    }
                }
                panic!("Invalid state");
            },
            request,
            move |game, player_index, _player_name, choice, _selected| {
                game.with_commands(player_index, |commands, game| {
                    gain_reward(commands, game, choice);
                });
            },
        )
    }

    fn add_units_request<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<CustomPhaseUnitsRequest> + 'static + Clone,
        units_selected: impl Fn(&mut Game, usize, &Vec<u32>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_multi_choice_reward_request_listener::<E, u32, CustomPhaseUnitsRequest, V>(
            event,
            priority,
            |r| &r.choices,
            CustomPhaseRequest::SelectUnits,
            |request, action| {
                if let CustomPhaseRequest::SelectUnits(request) = &request {
                    if let CustomPhaseEventAction::SelectUnits(choices) = action {
                        return (request.choices.clone(), choices, request.needed);
                    }
                }
                panic!("Invalid state");
            },
            request,
            move |game, player_index, _player_name, choice| {
                units_selected(game, player_index, choice);
            },
        )
    }

    fn add_choice_reward_request_listener<E, C, R, V>(
        self,
        event: E,
        priority: i32,
        get_choices: impl Fn(&R) -> &Vec<C> + 'static + Clone,
        to_request: impl Fn(R) -> CustomPhaseRequest + 'static + Clone,
        from_request: impl Fn(&CustomPhaseRequest, CustomPhaseEventAction) -> (Vec<C>, C)
            + 'static
            + Clone,
        request: impl Fn(&mut Game, usize, &V) -> Option<R> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &C, bool) + 'static + Clone,
    ) -> Self
    where
        C: Clone + PartialEq,
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        let g = gain_reward.clone();
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, player_name, details| {
                if let Some(r) = request(game, player_index, details) {
                    let choices = get_choices(&r);
                    if choices.is_empty() {
                        return None;
                    }
                    if choices.len() == 1 {
                        g(game, player_index, player_name, &choices[0], false);
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, player_index, player_name, action, request| {
                let (choices, selected) = from_request(&request, action);
                assert!(choices.contains(&selected), "Invalid choice");
                gain_reward(game, player_index, player_name, &selected, true);
            },
        )
    }

    fn add_multi_choice_reward_request_listener<E, C, R, V>(
        self,
        event: E,
        priority: i32,
        get_possible: impl Fn(&R) -> &Vec<C> + 'static + Clone,
        to_request: impl Fn(R) -> CustomPhaseRequest + 'static + Clone,
        from_request: impl Fn(&CustomPhaseRequest, CustomPhaseEventAction) -> (Vec<C>, Vec<C>, u8)
            + 'static
            + Clone,
        request: impl Fn(&mut Game, usize, &V) -> Option<R> + 'static + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &Vec<C>) + 'static + Clone,
    ) -> Self
    where
        C: Clone + PartialEq,
        E: Fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V> + 'static + Clone,
    {
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                if let Some(r) = request(game, player_index, details) {
                    let choices = get_possible(&r);
                    if choices.is_empty() {
                        return None;
                    }
                    return Some(to_request(r));
                }
                None
            },
            move |game, player_index, player_name, action, request| {
                let (choices, selected, needed) = from_request(&request, action);
                assert!(
                    selected.iter().all(|s| choices.contains(s)),
                    "Invalid choice"
                );
                assert_eq!(selected.len() as u8, needed, "Invalid choice count");
                gain_reward(game, player_index, player_name, &selected);
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

pub(crate) fn join_ability_initializers(setup: Vec<AbilityInitializer>) -> AbilityInitializer {
    Box::new(move |game: &mut Game, player_index: usize| {
        for initializer in &setup {
            initializer(game, player_index);
        }
    })
}
