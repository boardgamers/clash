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
    fn get_key(&self) -> String;

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.get_key();
        let deinitialize_event = event.clone();
        let initializer = move |game: &mut Game, player_index: usize| {
            let player = &mut game.players[player_index];
            player
                .event_listener_indices
                .entry(key.clone())
                .or_default()
                .push_back(
                    event(player.events.as_mut().expect("events should be set"))
                        .add_listener_mut(listener.clone(), priority),
                )
        };
        let key = self.get_key();
        let deinitializer = move |game: &mut Game, player_index: usize| {
            let player = &mut game.players[player_index];
            deinitialize_event(player.events.as_mut().expect("events should be set"))
                .remove_listener_mut(
                    player
                        .event_listener_indices
                        .entry(key.clone())
                        .or_default()
                        .pop_front()
                        .expect("tried to remove non-existing element"),
                )
        };
        self.add_ability_initializer(initializer)
            .add_ability_deinitializer(deinitializer)
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
        for initializer in setup.iter() {
            initializer(game, player_index)
        }
    })
}
