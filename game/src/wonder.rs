use crate::{
    events::EventMut,
    player_setup::{self, InitializerAndDeinitializer, PlayerSetup},
    Hexagon, Player, PlayerEvents, ResourcePile,
};

type PlacementChecker = Box<dyn Fn(&Hexagon) -> bool>;

pub struct Wonder {
    //* tho different wonders must not have the same name
    pub name: String,
    pub cost: ResourcePile,
    pub required_technologies: Vec<usize>,
    pub placement_requirement: Option<PlacementChecker>,
    pub builder: Option<Player>,
    pub initializer: PlayerSetup,
    pub deinitializer: PlayerSetup,
}

impl Wonder {
    pub fn create<F, G>(
        name: &str,
        cost: ResourcePile,
        required_technologies: Vec<usize>,
    ) -> WonderBuilder {
        WonderBuilder {
            name: name.to_string(),
            cost,
            required_technologies,
            placement_requirement: None,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    fn new(
        name: String,
        cost: ResourcePile,
        required_technologies: Vec<usize>,
        placement_requirement: Option<PlacementChecker>,
        initializer: PlayerSetup,
        deinitializer: PlayerSetup,
    ) -> Self {
        Self {
            name,
            cost,
            required_technologies,
            placement_requirement,
            builder: None,
            initializer,
            deinitializer,
        }
    }
}

pub struct WonderBuilder {
    name: String,
    cost: ResourcePile,
    required_technologies: Vec<usize>,
    placement_requirement: Option<PlacementChecker>,
    initializers: Vec<PlayerSetup>,
    deinitializers: Vec<PlayerSetup>,
}

impl WonderBuilder {
    pub fn placement_requirement(&mut self, placement_requirement: PlacementChecker) -> &mut Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    pub fn build(self) -> Wonder {
        let initializer = player_setup::join_player_setup(self.initializers);
        let deinitializer = player_setup::join_player_setup(self.deinitializers);
        Wonder::new(
            self.name,
            self.cost,
            self.required_technologies,
            self.placement_requirement,
            initializer,
            deinitializer,
        )
    }
}

impl InitializerAndDeinitializer for WonderBuilder {
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
        let name = self.name.clone();
        let deinitialize_event = event.clone();
        let initializer = Box::new(move |player: &mut Player| {
            player
                .wonder_event_listener_indices
                .entry(name.clone())
                .or_default()
                .push(event(&mut player.events).add_listener_mut(listener.clone(), priority))
        });
        let name = self.name.clone();
        let deinitializer = Box::new(move |player: &mut Player| {
            deinitialize_event(&mut player.events).remove_listener_mut(
                player
                    .wonder_event_listener_indices
                    .entry(name.clone())
                    .or_default()
                    .remove(0),
            )
        });
        self.add_initializer(initializer)
            .add_deinitializer(deinitializer)
    }
}
