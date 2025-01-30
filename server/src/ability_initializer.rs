use crate::action::Action;
use crate::content::custom_phase_actions::{
    CustomPhaseEvent, CustomPhaseEventAction, CustomPhaseEventType, CustomPhasePaymentRequest,
    CustomPhaseRequest,
};
use crate::resource_pile::ResourcePile;
use crate::{
    content::custom_actions::CustomActionType, events::EventMut, game::Game,
    player_events::PlayerEvents,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum EventOrigin {
    Advance(String),
    SpecialAdvance(String),
    Leader(String),
    Wonder(String),
}

impl EventOrigin {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            EventOrigin::Advance(name)
            | EventOrigin::SpecialAdvance(name)
            | EventOrigin::Wonder(name)
            | EventOrigin::Leader(name) => name,
        }
    }
}

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
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.get_key().clone();
        let deinitialize_event = event.clone();
        let initializer = move |game: &mut Game, player_index: usize| {
            event(
                game.players[player_index]
                    .events
                    .as_mut()
                    .expect("events should be set"),
            )
            .add_listener_mut(listener.clone(), priority, key.name().to_string());
        };
        let key = self.get_key().name().to_string();
        let deinitializer = move |game: &mut Game, player_index: usize| {
            deinitialize_event(
                game.players[player_index]
                    .events
                    .as_mut()
                    .expect("events should be set"),
            )
            .remove_listener_mut_by_key(&key);
        };
        self.add_ability_initializer(initializer)
            .add_ability_deinitializer(deinitializer)
    }

    fn add_state_change_event_listener<E>(
        self,
        event: E,
        priority: i32,
        start_custom_phase: impl Fn(&mut Game, usize, &str) -> Option<CustomPhaseRequest>
            + 'static
            + Clone, //return option<custom phase state>
        end_custom_phase: impl Fn(&mut Game, usize, &str, CustomPhaseEventAction, CustomPhaseRequest)
            + 'static
            + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<Game, usize, CustomPhaseEventType>
            + 'static
            + Clone,
    {
        let origin = self.get_key();
        self.add_player_event_listener(
            event,
            move |game, &player_index, event_type| {
                let s = &mut game.custom_phase_state;

                let player_name = game.players[player_index].get_name();

                if let Some(c) = s.current.as_ref() {
                    if let Some(action) = &c.response {
                        assert_eq!(&c.event_type, event_type);
                        if c.priority != priority {
                            // not our request
                            return;
                        }

                        let r = c.request.clone();
                        let a = action.clone();
                        s.current = None;
                        end_custom_phase(game, player_index, &player_name, a, r);
                    }
                    return;
                }

                if s.last_priority_used.is_some_and(|last| last < priority) {
                    // already handled before
                    return;
                }

                if let Some(request) = start_custom_phase(game, player_index, &player_name) {
                    let s = &mut game.custom_phase_state;
                    s.last_priority_used = Some(priority);
                    s.current = Some(CustomPhaseEvent {
                        event_type: event_type.clone(),
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

    #[allow(irrefutable_let_patterns)]
    fn add_payment_request_listener<E>(
        self,
        event: E,
        priority: i32,
        request: impl Fn(&mut Game, usize) -> Option<Vec<CustomPhasePaymentRequest>> + 'static + Clone, //return option<custom phase state>
        gain_reward: impl Fn(&mut Game, usize, &str, &Vec<ResourcePile>) + 'static + Clone,
    ) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<Game, usize, CustomPhaseEventType>
            + 'static
            + Clone,
    {
        self.add_state_change_event_listener(
            event,
            priority,
            move |game, player_index, _player_name| {
                request(game, player_index).map(CustomPhaseRequest::Payment)
            },
            move |game, player_index, player_name, action, request| {
                if let CustomPhaseRequest::Payment(requests) = &request {
                    if let CustomPhaseEventAction::Payment(payments) = action {
                        assert_eq!(requests.len(), payments.len());
                        for (request, payment) in requests.iter().zip(payments.iter()) {
                            let zero_payment = payment.is_empty() && request.optional;
                            assert!(
                                zero_payment || request.model.is_valid_payment(payment),
                                "Invalid payment"
                            );
                            game.players[player_index].loose_resources(payment.clone());
                        }
                        gain_reward(game, player_index, player_name, &payments);
                        return;
                    }
                }
                panic!("Invalid state");
            },
        )
    }

    fn add_once_per_turn_effect<P>(self, name: &str, pred: P) -> Self
    where
        P: Fn(&Action) -> bool + 'static + Clone,
    {
        let pred2 = pred.clone();
        let name2 = name.to_string();
        let name3 = name.to_string();
        self.add_player_event_listener(
            |event| &mut event.after_execute_action,
            move |player, action, ()| {
                if pred2(action) {
                    player.played_once_per_turn_effects.push(name2.to_string());
                }
            },
            0,
        )
        .add_player_event_listener(
            |event| &mut event.before_undo_action,
            move |player, action, ()| {
                if pred(action) {
                    player.played_once_per_turn_effects.retain(|a| a != &name3);
                }
            },
            0,
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
