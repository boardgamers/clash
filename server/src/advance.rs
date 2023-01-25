use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
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
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
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
    initializers: Vec<AbilityInitializer>,
    deinitializers: Vec<AbilityInitializer>,
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
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    pub fn with_advance_bonus(mut self, advance_bonus: AdvanceBonus) -> Self {
        self.advance_bonus = Some(advance_bonus);
        self
    }

    pub fn with_required_advance(mut self, required_advance: String) -> Self {
        self.required_advance = Some(required_advance);
        self
    }

    pub fn with_contradicting_advance(mut self, contradicting_advance: String) -> Self {
        self.contradicting_advance = Some(contradicting_advance);
        self
    }

    pub fn build(self) -> Advance {
        let initializer = ability_initializer::join_ability_initializers(self.initializers);
        let deinitializer = ability_initializer::join_ability_initializers(self.deinitializers);
        Advance {
            name: self.name,
            description: self.description,
            advance_bonus: self.advance_bonus,
            required_advance: self.required_advance,
            contradicting_advance: self.contradicting_advance,
            unlocked_building: self.unlocked_building,
            player_initializer: initializer,
            player_deinitializer: deinitializer,
        }
    }
}

impl AbilityInitializerSetup for AdvanceBuilder {
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
