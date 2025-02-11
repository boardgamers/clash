use crate::content::custom_phase_actions::{
    CurrentCustomPhaseEvent, CustomPhaseAdvanceRewardRequest, CustomPhaseEventAction,
    CustomPhasePaymentRequest, CustomPhaseRequest, CustomPhaseResourceRewardRequest,
};
use crate::events::{Event, EventOrigin};
use crate::game::UndoContext;
use crate::player_events::{CustomPhaseInfo, PlayerCommands};
use crate::resource_pile::ResourcePile;
use crate::{content::custom_actions::CustomActionType, game::Game, player_events::PlayerEvents};

pub(crate) type AbilityInitializer = Box<dyn Fn(&mut Game, usize)>;

pub(crate) trait AbilityInitializerSetup: Sized {
    fn add_ability_initializer<F>(self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static;
    fn add_ability_deinitializer<F>(self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static;
    fn add_one_time_ability_initializer<F>(self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static;
    fn add_ability_undo_deinitializer<F>(self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static;
    fn get_key(&self) -> EventOrigin;

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
        E: Fn(&mut PlayerEvents) -> &mut Event<Game, CustomPhaseInfo, V> + 'static + Clone,
    {
        let origin = self.get_key();
        self.add_player_event_listener(
            event,
            move |game, i, details| {
                let player_index = i.player;
                let player_name = game.players[player_index].get_name();

                if let Some(c) = &game.custom_phase_state.current.as_ref() {
                    if let Some(action) = &c.response {
                        assert_eq!(c.event_type, i.event_type);
                        if c.priority != priority {
                            // not our request
                            return;
                        }

                        let mut ctx = game.custom_phase_state.clone();
                        ctx.current.as_mut().expect("current missing").response = None;
                        game.undo_context_stack
                            .push(UndoContext::CustomPhaseEvent(ctx));
                        let r = c.request.clone();
                        let a = action.clone();
                        game.custom_phase_state.current = None;
                        end_custom_phase.clone()(game, player_index, &player_name, a, r);
                    }
                    return;
                }

                if game
                    .custom_phase_state
                    .last_priority_used
                    .is_some_and(|last| last < priority)
                {
                    // already handled before
                    return;
                }

                if let Some(request) = start_custom_phase(game, player_index, &player_name, details)
                {
                    let s = &mut game.custom_phase_state;
                    s.last_priority_used = Some(priority);
                    s.current = Some(CurrentCustomPhaseEvent {
                        event_type: i.event_type.clone(),
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
        request: impl Fn(&mut Game, usize, &V) -> Option<Vec<CustomPhasePaymentRequest>>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut PlayerCommands, &Game, &Vec<ResourcePile>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut Event<Game, CustomPhaseInfo, V> + 'static + Clone,
    {
        self.add_payment_request_listener(
            event,
            priority,
            request,
            move |game, player_index, _player_name, payments| {
                game.with_commands(player_index, true, |commands, game| {
                    gain_reward(commands, game, payments);
                });
            },
        )
    }

    fn add_payment_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<Vec<CustomPhasePaymentRequest>>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &Vec<ResourcePile>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut Event<Game, CustomPhaseInfo, V> + 'static + Clone,
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

    fn add_resource_reward_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<CustomPhaseResourceRewardRequest>
            + 'static
            + Clone,
        gain_reward_log: impl Fn(&Game, usize, &str, &ResourcePile, bool) -> String + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut Event<Game, CustomPhaseInfo, V> + 'static + Clone,
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
                        game.add_to_last_log_item(&g(game, player_index, &player_name, &r, false));
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
                        game.add_info_log_item(gain_reward_log(
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

    fn add_advance_reward_request_listener<E, V>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize, &V) -> Option<CustomPhaseAdvanceRewardRequest>
            + 'static
            + Clone,
        gain_reward: impl Fn(&mut Game, usize, &str, &str, bool) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut Event<Game, CustomPhaseInfo, V> + 'static + Clone,
    {
        let g = gain_reward.clone();
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name, details| {
                let req = request(game, player_index, details);
                if let Some(r) = &req {
                    if r.choices.len() == 1 {
                        let player_name = game.players[player_index].get_name();
                        g(game, player_index, &player_name, &r.choices[0], false);
                        return None;
                    }
                }
                req.map(CustomPhaseRequest::AdvanceReward)
            },
            move |game, player_index, _player_name, action, request| {
                if let CustomPhaseRequest::AdvanceReward(request) = &request {
                    if let CustomPhaseEventAction::AdvanceReward(reward) = action {
                        assert!(request.choices.contains(&reward), "Invalid advance");
                        gain_reward(
                            game,
                            player_index,
                            &game.players[player_index].get_name(),
                            &reward,
                            true,
                        );
                        return;
                    }
                }
                panic!("Invalid state");
            },
        )
    }

    fn add_custom_action(self, action: CustomActionType) -> Self {
        let deinitializer_action = action.clone();
        self.add_ability_initializer(move |game, player_index| {
            let player = &mut game.players[player_index];
            player.custom_actions.insert(action.clone());
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
