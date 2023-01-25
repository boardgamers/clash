use crate::ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup};

pub struct Leader {
    pub name: String,
    pub first_ability: String,
    pub first_ability_description: String,
    pub second_ability: String,
    pub second_ability_description: String,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
}

impl Leader {
    pub fn builder(
        name: &str,
        first_ability: &str,
        first_ability_description: &str,
        second_ability: &str,
        second_ability_description: &str,
    ) -> LeaderBuilder {
        LeaderBuilder::new(
            name.to_string(),
            first_ability.to_string(),
            first_ability_description.to_string(),
            second_ability.to_string(),
            second_ability_description.to_string(),
        )
    }

    fn new(
        name: String,
        first_ability: String,
        first_ability_description: String,
        second_ability: String,
        second_ability_description: String,
        player_initializer: AbilityInitializer,
        player_deinitializer: AbilityInitializer,
    ) -> Self {
        Self {
            name,
            first_ability,
            first_ability_description,
            second_ability,
            second_ability_description,
            player_initializer,
            player_deinitializer,
        }
    }
}

pub struct LeaderBuilder {
    name: String,
    first_ability: String,
    first_ability_description: String,
    second_ability: String,
    second_ability_description: String,
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
}

impl LeaderBuilder {
    fn new(
        name: String,
        first_ability: String,
        first_ability_description: String,
        second_ability: String,
        second_ability_description: String,
    ) -> Self {
        Self {
            name,
            first_ability,
            first_ability_description,
            second_ability,
            second_ability_description,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
        }
    }

    pub fn build(self) -> Leader {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        Leader::new(
            self.name,
            self.first_ability,
            self.first_ability_description,
            self.second_ability,
            self.second_ability_description,
            player_initializer,
            player_deinitializer,
        )
    }
}

impl AbilityInitializerSetup for LeaderBuilder {
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