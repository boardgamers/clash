use std::fmt::Display;

use crate::player::{self, PlayerInitializer, PlayerSetup};

pub struct SpecialAdvance {
    pub name: String,
    pub description: String,
    pub required_advance: String,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl SpecialAdvance {
    pub fn builder(name: &str, required_advance: &str) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(name.to_string(), required_advance.to_string())
    }

    fn new(
        name: String,
        description: String,
        required_advance: String,
        player_initializer: PlayerInitializer,
        player_deinitializer: PlayerInitializer,
    ) -> Self {
        Self {
            name,
            description,
            required_advance,
            player_initializer,
            player_deinitializer,
        }
    }
}

pub struct SpecialAdvanceBuilder {
    name: String,
    descriptions: Vec<String>,
    required_advance: String,
    initializers: Vec<PlayerInitializer>,
    deinitializers: Vec<PlayerInitializer>,
}

impl SpecialAdvanceBuilder {
    fn new(name: String, required_advance: String) -> Self {
        Self {
            name,
            descriptions: Vec::new(),
            required_advance,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    pub fn add_description(mut self, description: String) -> Self {
        self.descriptions.push(description);
        self
    }

    pub fn build(self) -> SpecialAdvance {
        let initializer = player::join_player_initializers(self.initializers);
        let deinitializer = player::join_player_initializers(self.deinitializers);
        SpecialAdvance::new(
            self.name,
            String::from("● ") + &self.descriptions.join("\n● "),
            self.required_advance,
            initializer,
            deinitializer,
        )
    }
}

impl Display for SpecialAdvanceBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for SpecialAdvanceBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.deinitializers.push(deinitializer);
        self
    }
}
