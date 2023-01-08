use std::fmt::Display;

use crate::{
    player::{self, PlayerInitializer, PlayerSetup},
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
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl Advance {
    pub fn builder(
        name: &str,
        description: &str,
        advance_bonus: Option<AdvanceBonus>,
    ) -> AdvanceBuilder {
        AdvanceBuilder::new(name.to_string(), description.to_string(), advance_bonus)
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
    initializers: Vec<PlayerInitializer>,
    deinitializers: Vec<PlayerInitializer>,
}

impl AdvanceBuilder {
    fn new(name: String, description: String, advance_bonus: Option<AdvanceBonus>) -> Self {
        Self {
            name,
            description,
            advance_bonus,
            required_advance: None,
            contradicting_advance: None,
            unlocked_building: None,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
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
        let initializer = player::join_player_initializers(self.initializers);
        let deinitializer = player::join_player_initializers(self.deinitializers);
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

impl Display for AdvanceBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for AdvanceBuilder {
    fn add_player_initializer(mut self, initializer: PlayerInitializer) -> Self {
        self.initializers.push(initializer);
        self
    }

    fn add_player_deinitializer(mut self, deinitializer: PlayerInitializer) -> Self {
        self.deinitializers.push(deinitializer);
        self
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
