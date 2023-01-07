use std::fmt::Display;

use crate::{
    player::{self, PlayerInitializer, PlayerSetup},
    resource_pile::ResourcePile,
};

use AdvanceBonus::*;

pub struct Technology {
    pub name: String,
    pub description: String,
    pub advance_bonus: Option<AdvanceBonus>,
    pub required_technology: Option<String>,
    pub contradicting_technology: Option<String>,
    pub unlocked_building: Option<String>,
    pub player_initializer: PlayerInitializer,
    pub player_deinitializer: PlayerInitializer,
}

impl Technology {
    pub fn builder(
        name: &str,
        description: &str,
        advance_bonus: Option<AdvanceBonus>,
    ) -> TechnologyBuilder {
        TechnologyBuilder::new(name.to_string(), description.to_string(), advance_bonus)
    }

    fn new(
        name: String,
        description: String,
        advance_bonus: Option<AdvanceBonus>,
        required_technology: Option<String>,
        contradicting_technology: Option<String>,
        unlocked_building: Option<String>,
        player_initializer: PlayerInitializer,
        player_deinitializer: PlayerInitializer,
    ) -> Self {
        Self {
            name,
            description,
            advance_bonus,
            required_technology,
            contradicting_technology,
            unlocked_building,
            player_initializer,
            player_deinitializer,
        }
    }
}

impl PartialEq for Technology {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub struct TechnologyBuilder {
    name: String,
    description: String,
    advance_bonus: Option<AdvanceBonus>,
    required_technology: Option<String>,
    contradicting_technology: Option<String>,
    unlocked_building: Option<String>,
    initializers: Vec<PlayerInitializer>,
    deinitializers: Vec<PlayerInitializer>,
}

impl TechnologyBuilder {
    fn new(name: String, description: String, advance_bonus: Option<AdvanceBonus>) -> Self {
        Self {
            name,
            description,
            advance_bonus,
            required_technology: None,
            contradicting_technology: None,
            unlocked_building: None,
            initializers: Vec::new(),
            deinitializers: Vec::new(),
        }
    }

    pub fn with_required_technology(mut self, required_technology: String) -> Self {
        self.required_technology = Some(required_technology);
        self
    }

    pub fn with_contradicting_technology(mut self, contradicting_technology: String) -> Self {
        self.contradicting_technology = Some(contradicting_technology);
        self
    }

    pub fn build(self) -> Technology {
        let initializer = player::join_player_initializers(self.initializers);
        let deinitializer = player::join_player_initializers(self.deinitializers);
        Technology::new(
            self.name,
            self.description,
            self.advance_bonus,
            self.required_technology,
            self.contradicting_technology,
            self.unlocked_building,
            initializer,
            deinitializer,
        )
    }
}

impl Display for TechnologyBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.clone())
    }
}

impl PlayerSetup for TechnologyBuilder {
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
