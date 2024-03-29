use crate::{
    content::custom_actions::CustomActionType, events::EventMut, game::Game, map::Terrain,
    player_events::PlayerEvents, resource_pile::ResourcePile, utils,
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

    fn add_collect_option(self, terrain: Terrain, option: ResourcePile) -> Self {
        let deinitializer_terrain = terrain.clone();
        let deinitializer_option = option.clone();
        self.add_one_time_ability_initializer(move |game, player_index| {
            let player = &mut game.players[player_index];
            player
                .collect_options
                .entry(terrain.clone())
                .or_default()
                .push(option.clone());
        })
        .add_ability_undo_deinitializer(move |game, player_index| {
            let player = &mut game.players[player_index];
            utils::remove_element(
                player
                    .collect_options
                    .get_mut(&deinitializer_terrain)
                    .expect("player should have options for terrain type"),
                &deinitializer_option,
            );
            //*Note that this will break if multiple effects add the same collect option
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
