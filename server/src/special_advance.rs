use crate::ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup};

pub struct SpecialAdvance {
    pub name: String,
    pub description: String,
    pub required_advance: String,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub player_one_time_initializer: AbilityInitializer,
}

impl SpecialAdvance {
    pub fn builder(name: &str, required_advance: &str) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(name.to_string(), required_advance.to_string())
    }

    fn new(
        name: String,
        description: String,
        required_advance: String,
        player_initializer: AbilityInitializer,
        player_deinitializer: AbilityInitializer,
        player_one_time_initializer: AbilityInitializer,
    ) -> Self {
        Self {
            name,
            description,
            required_advance,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
        }
    }
}

pub struct SpecialAdvanceBuilder {
    name: String,
    descriptions: Vec<String>,
    required_advance: String,
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
    player_one_time_initializers: Vec<AbilityInitializer>,
}

impl SpecialAdvanceBuilder {
    fn new(name: String, required_advance: String) -> Self {
        Self {
            name,
            descriptions: Vec::new(),
            required_advance,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
            player_one_time_initializers: Vec::new(),
        }
    }

    pub fn add_description(mut self, description: String) -> Self {
        self.descriptions.push(description);
        self
    }

    pub fn build(self) -> SpecialAdvance {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        let player_one_time_initializer =
            ability_initializer::join_ability_initializers(self.player_one_time_initializers);
        SpecialAdvance::new(
            self.name,
            String::from("● ") + &self.descriptions.join("\n● "),
            self.required_advance,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
        )
    }
}

impl AbilityInitializerSetup for SpecialAdvanceBuilder {
    fn add_ability_initializer(mut self, initializer: AbilityInitializer) -> Self {
        self.player_initializers.push(initializer);
        self
    }

    fn add_ability_deinitializer(mut self, deinitializer: AbilityInitializer) -> Self {
        self.player_deinitializers.push(deinitializer);
        self
    }

    fn add_ability_one_time_ability_initializer(mut self, initializer: AbilityInitializer) -> Self {
        self.player_one_time_initializers.push(initializer);
        self
    }

    fn get_key(&self) -> String {
        self.name.clone()
    }
}
