use std::fmt::Display;

use crate::{
    hexagon::Position,
    player::{self, PlayerInitializer, PlayerSetup},
    resource_pile::ResourcePile,
};

//todo! provide more info for position
type PlacementChecker = Box<dyn Fn(&Position) -> bool>;

pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: ResourcePile,
    pub required_advances: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
    pub builder: Option<String>,
}

impl Wonder {
    pub fn builder(
        name: &str,
        cost: ResourcePile,
        required_advances: Vec<&str>,
    ) -> WonderBuilder {
        WonderBuilder::new(
            name.to_string(),
            cost,
            required_advances
                .into_iter()
                .map(|name| name.to_string())
                .collect(),
        )
    }
}

pub struct WonderBuilder {
    name: String,
    descriptions: Vec<String>,
    cost: ResourcePile,
    required_advance: Vec<String>,
    placement_requirement: Option<PlacementChecker>,
    player_initializers: Vec<PlayerInitializer>,
    player_deinitializers: Vec<PlayerInitializer>,
}

impl WonderBuilder {
    fn new(name: String, cost: ResourcePile, required_advances: Vec<String>) -> Self {
        Self {
            name,
            descriptions: Vec::new(),
            cost,
            required_advance: required_advances,
            placement_requirement: None,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
        }
    }

    pub fn add_description(mut self, description: String) -> Self {
        self.descriptions.push(description);
        self
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
            description: String::from("● ") + &self.descriptions.join("\n● "),
            cost: self.cost,
            required_advances: self.required_advance,
            placement_requirement: self.placement_requirement,
            player_initializer,
            player_deinitializer,
            builder: None,
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
