use crate::{
    events::EventMut,
    player_setup::{self, InitializerAndDeinitializer, PlayerSetup},
    Player, PlayerEvents,
};

pub struct Leader {
    pub name: String,
    pub initializer: PlayerSetup,
    pub deinitializer: PlayerSetup,
}

impl Leader {
    pub fn create(name: &str) -> LeaderBuilder {
        LeaderBuilder {
            name: name.to_string(),
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    fn new(name: String, initializer: PlayerSetup, deinitializer: PlayerSetup) -> Self {
        Self {
            name,
            initializer,
            deinitializer,
        }
    }
}

pub struct LeaderBuilder {
    name: String,
    initializers: Vec<PlayerSetup>,
    deinitializers: Vec<PlayerSetup>,
}

impl LeaderBuilder {
    pub fn build(self) -> Leader {
        let initializer = player_setup::join_player_setup(self.initializers);
        let deinitializer = player_setup::join_player_setup(self.deinitializers);
        Leader::new(self.name, initializer, deinitializer)
    }
}

impl InitializerAndDeinitializer for LeaderBuilder {
    fn add_initializer(mut self, initializer: PlayerSetup) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_deinitializer(mut self, deinitializer: PlayerSetup) -> Self {
        self.deinitializers.push(deinitializer);
        self
    }

    fn add_event_listener<T, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T> + 'static + Clone,
        F: Fn(&mut T) + 'static + Clone,
    {
        let deinitialize_event = event.clone();
        let initializer = Box::new(move |player: &mut Player| {
            player
                .leader_event_listener_indices
                .push(event(&mut player.events).add_listener_mut(listener.clone(), priority))
        });
        let deinitializer = Box::new(move |player: &mut Player| {
            deinitialize_event(&mut player.events)
                .remove_listener_mut(player.leader_event_listener_indices.remove(0))
        });
        self.add_initializer(initializer)
            .add_deinitializer(deinitializer)
    }
}
