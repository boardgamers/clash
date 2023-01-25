use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
    hexagon::Position,
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
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub builder: Option<usize>,
}

impl Wonder {
    pub fn builder(name: &str, cost: ResourcePile, required_advances: Vec<&str>) -> WonderBuilder {
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
    required_advances: Vec<String>,
    placement_requirement: Option<PlacementChecker>,
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
}

impl WonderBuilder {
    fn new(name: String, cost: ResourcePile, required_advances: Vec<String>) -> Self {
        Self {
            name,
            descriptions: Vec::new(),
            cost,
            required_advances,
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
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        Wonder {
            name: self.name,
            description: String::from("● ") + &self.descriptions.join("\n● "),
            cost: self.cost,
            required_advances: self.required_advances,
            placement_requirement: self.placement_requirement,
            player_initializer,
            player_deinitializer,
            builder: None,
        }
    }
}

impl AbilityInitializerSetup for WonderBuilder {
    fn add_ability_initializer(mut self, initializer: AbilityInitializer) -> Self {
        self.player_initializers.push(initializer);
        self
    }

    fn add_ability_deinitializer(mut self, deinitializer: AbilityInitializer) -> Self {
        self.player_deinitializers.push(deinitializer);
        self
    }

    fn key(&self) -> String {
        self.name.clone()
    }
}
