use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
    game::Game,
};

pub struct SpecialAdvance {
    pub name: String,
    pub description: String,
    pub required_advance: String,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub player_one_time_initializer: AbilityInitializer,
    pub player_undo_deinitializer: AbilityInitializer,
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
        player_undo_deinitializer: AbilityInitializer,
    ) -> Self {
        Self {
            name,
            description,
            required_advance,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
            player_undo_deinitializer,
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
    player_undo_deinitializer: Vec<AbilityInitializer>,
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
            player_undo_deinitializer: Vec::new(),
        }
    }

    pub fn add_description(mut self, description: &str) -> Self {
        self.descriptions.push(description.to_string());
        self
    }

    pub fn build(self) -> SpecialAdvance {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        let player_one_time_initializer =
            ability_initializer::join_ability_initializers(self.player_one_time_initializers);
        let player_undo_deinitializer =
            ability_initializer::join_ability_initializers(self.player_undo_deinitializer);
        SpecialAdvance::new(
            self.name,
            String::from("✦ ") + &self.descriptions.join("\n✦ "),
            self.required_advance,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
            player_undo_deinitializer,
        )
    }
}

impl AbilityInitializerSetup for SpecialAdvanceBuilder {
    fn add_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_initializers.push(Box::new(initializer));
        self
    }

    fn add_ability_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_deinitializers.push(Box::new(deinitializer));
        self
    }

    fn add_one_time_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_one_time_initializers
            .push(Box::new(initializer));
        self
    }

    fn add_ability_undo_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_undo_deinitializer.push(Box::new(deinitializer));
        self
    }

    fn get_key(&self) -> String {
        self.name.clone()
    }
}
