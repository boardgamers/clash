pub const WONDER_VICTORY_POINTS: f32 = 4.0;

use std::fmt::Display;

use crate::{
    hexagon::Hexagon,
    player::{self, PlayerInitializer, PlayerSetup},
    resource_pile::ResourcePile,
};

type PlacementChecker = Box<dyn Fn(&Hexagon) -> bool>;

pub struct Wonder {
    pub name: String,
    pub cost: ResourcePile,
    pub required_technologies: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl Wonder {
    pub fn builder(
        name: &str,
        cost: ResourcePile,
        required_technologies: Vec<&str>,
    ) -> WonderBuilder {
        WonderBuilder::new(
            name.to_string(),
            cost,
            required_technologies
                .into_iter()
                .map(|name| name.to_string())
                .collect(),
        )
    }
}

pub struct WonderBuilder {
    name: String,
    cost: ResourcePile,
    required_technologies: Vec<String>,
    placement_requirement: Option<PlacementChecker>,
    player_initializers: Vec<PlayerInitializer>,
    player_deinitializers: Vec<PlayerInitializer>,
}

impl WonderBuilder {
    fn new(name: String, cost: ResourcePile, required_technologies: Vec<String>) -> Self {
        Self {
            name,
            cost,
            required_technologies,
            placement_requirement: None,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
        }
    }

    pub fn placement_requirement(&mut self, placement_requirement: PlacementChecker) -> &mut Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    pub fn build(self) -> Wonder {
        let player_initializer = player::join_player_initializers(self.player_initializers);
        let player_deinitializer = player::join_player_initializers(self.player_deinitializers);
        Wonder {
            name: self.name,
            cost: self.cost,
            required_technologies: self.required_technologies,
            placement_requirement: self.placement_requirement,
            player_initializer,
            player_deinitializer,
        }
    }
}

impl Display for WonderBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for WonderBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.player_initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.player_deinitializers.push(deinitializer);
        self
    }
}
