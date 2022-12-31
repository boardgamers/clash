use std::fmt::Display;

use crate::player::{self, PlayerInitializer, PlayerSetup};

pub struct SpecialTechnology {
    pub name: String,
    pub required_technology: String,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl SpecialTechnology {
    pub fn builder(name: &str, required_technology: &str) -> SpecialTechnologyBuilder {
        SpecialTechnologyBuilder::new(name.to_string(), required_technology.to_string())
    }

    fn new(
        name: String,
        required_technology: String,
        player_initializer: PlayerInitializer,
        player_deinitializer: PlayerInitializer,
    ) -> Self {
        Self {
            name,
            required_technology,
            player_initializer,
            player_deinitializer,
        }
    }
}

pub struct SpecialTechnologyBuilder {
    name: String,
    required_technology: String,
    initializers: Vec<PlayerInitializer>,
    deinitializers: Vec<PlayerInitializer>,
}

impl SpecialTechnologyBuilder {
    fn new(name: String, required_technology: String) -> Self {
        Self {
            name,
            required_technology,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    pub fn build(self) -> SpecialTechnology {
        let initializer = player::join_player_initializers(self.initializers);
        let deinitializer = player::join_player_initializers(self.deinitializers);
        SpecialTechnology::new(
            self.name,
            self.required_technology,
            initializer,
            deinitializer,
        )
    }
}

impl Display for SpecialTechnologyBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for SpecialTechnologyBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.deinitializers.push(deinitializer);
        self
    }
}
