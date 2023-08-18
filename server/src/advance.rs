use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
    game::Game,
    resource_pile::ResourcePile,
};

use AdvanceBonus::*;

pub struct Advance {
    pub name: String,
    pub description: String,
    pub advance_bonus: Option<AdvanceBonus>,
    pub required_advance: Option<String>,
    pub contradicting_advance: Option<String>,
    pub unlocked_building: Option<String>,
    pub government: Option<String>,
    pub government_tier: Option<u8>,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub player_one_time_initializer: AbilityInitializer,
    pub player_undo_deinitializer: AbilityInitializer,
}

impl Advance {
    pub fn builder(name: &str, description: &str) -> AdvanceBuilder {
        AdvanceBuilder::new(name.to_string(), description.to_string())
    }
}

impl PartialEq for Advance {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub struct AdvanceBuilder {
    name: String,
    description: String,
    advance_bonus: Option<AdvanceBonus>,
    required_advance: Option<String>,
    contradicting_advance: Option<String>,
    unlocked_building: Option<String>,
    government: Option<String>,
    government_tier: Option<u8>,
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
    player_one_time_initializers: Vec<AbilityInitializer>,
    player_undo_deinitializers: Vec<AbilityInitializer>,
}

impl AdvanceBuilder {
    fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            advance_bonus: None,
            required_advance: None,
            contradicting_advance: None,
            unlocked_building: None,
            government: None,
            government_tier: None,
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
            player_one_time_initializers: Vec::new(),
            player_undo_deinitializers: Vec::new(),
        }
    }

    pub fn with_advance_bonus(mut self, advance_bonus: AdvanceBonus) -> Self {
        self.advance_bonus = Some(advance_bonus);
        self
    }

    pub fn with_required_advance(mut self, required_advance: &str) -> Self {
        self.required_advance = Some(required_advance.to_string());
        self
    }

    pub fn with_contradicting_advance(mut self, contradicting_advance: &str) -> Self {
        self.contradicting_advance = Some(contradicting_advance.to_string());
        self
    }

    pub fn with_unlocked_building(mut self, unlocked_building: &str) -> Self {
        self.unlocked_building = Some(unlocked_building.to_string());
        self
    }

    pub fn leading_government_advance(mut self, government: &str) -> Self {
        self.government = Some(government.to_string());
        self.government_tier = Some(0);
        self
    }

    //* tier should start with 0
    pub fn government_advance(mut self, tier: u8) -> Self {
        self.government_tier = Some(tier);
        self
    }

    pub fn build(self) -> Advance {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        let player_one_time_initializer =
            ability_initializer::join_ability_initializers(self.player_one_time_initializers);
        let player_undo_deinitializer =
            ability_initializer::join_ability_initializers(self.player_undo_deinitializers);
        Advance {
            name: self.name,
            description: self.description,
            advance_bonus: self.advance_bonus,
            required_advance: self.required_advance,
            contradicting_advance: self.contradicting_advance,
            unlocked_building: self.unlocked_building,
            government: self.government,
            government_tier: self.government_tier,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
            player_undo_deinitializer,
        }
    }
}

impl AbilityInitializerSetup for AdvanceBuilder {
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
        self.player_undo_deinitializers
            .push(Box::new(deinitializer));
        self
    }

    fn get_key(&self) -> String {
        self.name.clone()
    }
}

pub enum AdvanceBonus {
    MoodToken,
    CultureToken,
}

impl AdvanceBonus {
    pub fn resources(&self) -> ResourcePile {
        match self {
            MoodToken => ResourcePile::mood_tokens(1),
            CultureToken => ResourcePile::culture_tokens(1),
        }
    }
}
