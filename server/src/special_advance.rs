use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::events::EventOrigin;
use crate::game::Game;
use enumset::EnumSetType;
use serde::{Deserialize, Serialize};

#[derive(EnumSetType, Serialize, Deserialize, Debug, Ord, PartialOrd, Hash)]
pub enum SpecialAdvance {
    // Maya
    // Terrace,

    // Rome
    Aqueduct,
    RomanRoads,
    Captivi,
    Provinces,

    // Greece
    Study,
    Sparta,
    HellenisticCulture,
    CityStates,

    // China
    RiceCultivation,
    Expansion,
    Fireworks,
    ImperialArmy,
    
    // Vikings
    ShipConstruction,
    Longships,
    Raiding,
    RuneStones,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
impl SpecialAdvance {
    #[must_use]
    pub fn info<'a>(&self, game: &'a Game) -> &'a SpecialAdvanceInfo {
        game.cache.get_special_advance(*self)
    }

    #[must_use]
    pub fn name<'a>(&self, game: &'a Game) -> &'a str {
        self.info(game).name.as_str()
    }
}

#[derive(Clone)]
pub enum SpecialAdvanceRequirement {
    AnyGovernment,
    Advance(Advance),
}

#[derive(Clone)]
pub struct SpecialAdvanceInfo {
    pub advance: SpecialAdvance,
    pub name: String,
    pub description: String,
    pub requirement: SpecialAdvanceRequirement,
    pub listeners: AbilityListeners,
}

impl SpecialAdvanceInfo {
    #[must_use]
    pub fn builder(
        advance: SpecialAdvance,
        requirement: SpecialAdvanceRequirement,
        name: &str,
        description: &str,
    ) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(
            advance,
            requirement,
            name.to_string(),
            description.to_string(),
        )
    }

    fn new(
        advance: SpecialAdvance,
        name: String,
        description: String,
        requirement: SpecialAdvanceRequirement,
        listeners: AbilityListeners,
    ) -> Self {
        Self {
            advance,
            name,
            description,
            requirement,
            listeners,
        }
    }
}

pub struct SpecialAdvanceBuilder {
    advance: SpecialAdvance,
    name: String,
    description: String,
    requirement: SpecialAdvanceRequirement,
    builder: AbilityInitializerBuilder,
}

impl SpecialAdvanceBuilder {
    fn new(
        advance: SpecialAdvance,
        requirement: SpecialAdvanceRequirement,
        name: String,
        description: String,
    ) -> Self {
        Self {
            advance,
            name,
            description,
            requirement,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn build(self) -> SpecialAdvanceInfo {
        SpecialAdvanceInfo::new(
            self.advance,
            self.name,
            self.description,
            self.requirement,
            self.builder.build(),
        )
    }
}

impl AbilityInitializerSetup for SpecialAdvanceBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::SpecialAdvance(self.advance)
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}
