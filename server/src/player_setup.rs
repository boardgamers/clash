use crate::{events::EventMut, Player, PlayerEvents};

pub type PlayerSetup = Box<dyn Fn(&mut Player)>;

pub trait Initializer {
    fn add_initializer(self, initializer: PlayerSetup) -> Self;
    fn add_event_listener<T, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        Self: Sized,
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T> + 'static + Clone,
        F: Fn(&mut T) + 'static + Clone,
    {
        let initializer = Box::new(move |player: &mut Player| {
            event(&mut player.events).add_listener_mut(listener.clone(), priority);
        });
        self.add_initializer(initializer)
    }
}

pub trait InitializerAndDeinitializer {
    fn add_initializer(self, initializer: PlayerSetup) -> Self;
    fn add_deinitializer(self, deinitializer: PlayerSetup) -> Self;
    fn add_event_listener<T, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T> + 'static + Clone,
        F: Fn(&mut T) + 'static + Clone;
}

pub fn join_player_setup(initializers: Vec<PlayerSetup>) -> PlayerSetup {
    Box::new(move |player: &mut Player| {
        for initializer in initializers.iter() {
            initializer(player)
        }
    })
}
