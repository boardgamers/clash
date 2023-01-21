use crate::ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup};

pub struct SpecialAdvance {
    pub name: String,
    pub description: String,
    pub required_advance: String,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
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
    initializers: Vec<AbilityInitializer>,
    deinitializers: Vec<AbilityInitializer>,
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
        let initializer = ability_initializer::join_ability_initializers(self.initializers);
        let deinitializer = ability_initializer::join_ability_initializers(self.deinitializers);
        SpecialAdvance::new(
            self.name,
            String::from("● ") + &self.descriptions.join("\n● "),
            self.required_advance,
            initializer,
            deinitializer,
        )
    }
}

impl AbilityInitializerSetup for SpecialAdvanceBuilder {
    fn add_ability_initializer(mut self, initializer: AbilityInitializer) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_ability_deinitializer(mut self, deinitializer: AbilityInitializer) -> Self {
        self.deinitializers.push(deinitializer);
        self
    }

    fn key(&self) -> String {
        self.name.clone()
    }
}
