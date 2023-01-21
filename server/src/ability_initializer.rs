use crate::{events::EventMut, player_events::PlayerEvents, game::Game};

pub type AbilityInitializer = Box<dyn Fn(&mut Game, usize)>;

pub trait AbilityInitializerSetup: Sized {
    fn add_player_initializer(self, initializer: AbilityInitializer) -> Self;
    fn add_player_deinitializer(self, deinitializer: AbilityInitializer) -> Self;
    fn key(&self) -> String;

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.key();
        let deinitialize_event = event.clone();
        let initializer = Box::new(move |game: &mut Game, player: usize| {
            let player = &mut game.players[player];
            player
                .event_listener_indices
                .entry(key.clone())
                .or_default()
                .push_back(
                    event(
                        player
                            .events
                            .as_mut()
                            .expect("Events should be set after use"),
                    )
                    .add_listener_mut(listener.clone(), priority),
                )
        });
        let key = self.key();
        let deinitializer = Box::new(move |game: &mut Game, player: usize| {
            let player = &mut game.players[player];
            deinitialize_event(
                player
                    .events
                    .as_mut()
                    .expect("Events should be set after use"),
            )
            .remove_listener_mut(
                player
                    .event_listener_indices
                    .entry(key.clone())
                    .or_default()
                    .pop_front()
                    .unwrap_or_else(|| panic!("{}: tried to remove non-existing element", key)),
            )
        });
        self.add_player_initializer(initializer)
            .add_player_deinitializer(deinitializer)
    }

    fn add_custom_action(self, action: &str) -> Self {
        let action = action.to_string();
        self.add_player_initializer(Box::new(move |game: &mut Game, player: usize| {
            let player = &mut game.players[player];
            player.custom_actions.push(action.clone())
        }))
    }
}

pub fn join_ability_initializers(setup: Vec<AbilityInitializer>) -> AbilityInitializer {
    Box::new(move |game: &mut Game, player: usize| {
        for initializer in setup.iter() {
            initializer(game, player)
        }
    })
}
