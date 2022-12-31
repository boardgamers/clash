use std::fmt::Display;

use crate::player::{self, PlayerInitializer, PlayerSetup};

pub struct Leader {
    pub name: String,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl Leader {
    pub fn builder(name: &str) -> LeaderBuilder {
        LeaderBuilder::new(name.to_string())
    }

    fn new(
        name: String,
        player_initializer: PlayerInitializer,
        player_deinitializer: PlayerInitializer,
    ) -> Self {
        Self {
            name,
            player_initializer,
            player_deinitializer,
        }
    }
}

pub struct LeaderBuilder {
    name: String,
    player_initializers: Vec<PlayerInitializer>,
    player_deinitializers: Vec<PlayerInitializer>,
}

impl LeaderBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
        }
    }

    pub fn build(self) -> Leader {
        let player_initializer = player::join_player_initializers(self.player_initializers);
        let player_deinitializer = player::join_player_initializers(self.player_deinitializers);
        Leader::new(self.name, player_initializer, player_deinitializer)
    }
}

impl Display for LeaderBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for LeaderBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.player_initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.player_deinitializers.push(deinitializer);
        self
    }
}
