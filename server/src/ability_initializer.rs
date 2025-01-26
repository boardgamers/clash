use crate::action::Action;
use crate::{
    content::custom_actions::CustomActionType, events::EventMut, game::Game,
    player_events::PlayerEvents,
};

pub type AbilityInitializer = Box<dyn Fn(&mut Game, usize)>;

pub trait AbilityInitializerSetup: Sized {
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
    fn get_key(&self) -> String;

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        T: Clone + PartialEq,
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.get_key();
        let deinitialize_event = event.clone();
        let initializer = move |game: &mut Game, player_index: usize| {
            event(
                game.players[player_index]
                    .events
                    .as_mut()
                    .expect("events should be set"),
            )
            .add_listener_mut(listener.clone(), priority, key.clone());
        };
        let key = self.get_key();
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

pub fn join_ability_initializers(setup: Vec<AbilityInitializer>) -> AbilityInitializer {
    Box::new(move |game: &mut Game, player_index: usize| {
        for initializer in &setup {
            initializer(game, player_index);
        }
    })
}
