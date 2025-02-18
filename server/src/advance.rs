use crate::{ability_initializer::AbilityInitializerSetup, resource_pile::ResourcePile};

use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::city_pieces::Building;
use crate::events::EventOrigin;
use Bonus::*;

pub struct Advance {
    pub name: String,
    pub description: String,
    pub bonus: Option<Bonus>,
    pub required: Option<String>,
    pub contradicting: Vec<String>,
    pub unlocked_building: Option<Building>,
    pub government: Option<String>,
    pub listeners: AbilityListeners,
}

impl Advance {
    #[must_use]
    pub(crate) fn builder(name: &str, description: &str) -> AdvanceBuilder {
        AdvanceBuilder::new(name.to_string(), description.to_string())
    }
}

impl PartialEq for Advance {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub(crate) struct AdvanceBuilder {
    pub name: String,
    description: String,
    advance_bonus: Option<Bonus>,
    pub required_advance: Option<String>,
    contradicting_advance: Vec<String>,
    unlocked_building: Option<Building>,
    government: Option<String>,
    builder: AbilityInitializerBuilder,
}

impl AdvanceBuilder {
    fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            advance_bonus: None,
            required_advance: None,
            contradicting_advance: vec![],
            unlocked_building: None,
            government: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn with_advance_bonus(mut self, advance_bonus: Bonus) -> Self {
        self.advance_bonus = Some(advance_bonus);
        self
    }

    #[must_use]
    pub fn with_required_advance(mut self, required_advance: &str) -> Self {
        self.required_advance = Some(required_advance.to_string());
        self
    }

    #[must_use]
    pub fn with_contradicting_advance(mut self, contradicting_advance: &[&str]) -> Self {
        self.contradicting_advance = contradicting_advance
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        self
    }

    #[must_use]
    pub fn with_unlocked_building(mut self, unlocked_building: Building) -> Self {
        self.unlocked_building = Some(unlocked_building);
        self
    }

    #[must_use]
    pub fn with_government(mut self, government: &str) -> Self {
        self.government = Some(government.to_string());
        self
    }

    #[must_use]
    pub fn build(self) -> Advance {
        Advance {
            name: self.name,
            description: self.description,
            bonus: self.advance_bonus,
            required: self.required_advance,
            contradicting: self.contradicting_advance,
            unlocked_building: self.unlocked_building,
            government: self.government,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for AdvanceBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Advance(self.name.clone())
    }
}

pub enum Bonus {
    MoodToken,
    CultureToken,
}

impl Bonus {
    #[must_use]
    pub fn resources(&self) -> ResourcePile {
        match self {
            MoodToken => ResourcePile::mood_tokens(1),
            CultureToken => ResourcePile::culture_tokens(1),
        }
    }
}
