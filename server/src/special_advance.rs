use enumset::EnumSetType;
use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::events::EventOrigin;
use crate::game::Game;
use serde::{Deserialize, Serialize};

#[derive(EnumSetType, Serialize, Deserialize, Debug, Ord, PartialOrd, Hash)]
pub enum SpecialAdvance {
    // Maya
    Terrace,

    // Rome
    Aqueduct,
    RomanRoads,
    Captivi,
    Provinces,
}

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
pub struct SpecialAdvanceInfo {
    pub advance: SpecialAdvance,
    pub name: String,
    pub description: String,
    pub required_advance: Advance,
    pub listeners: AbilityListeners,
}

impl SpecialAdvanceInfo {
    #[must_use]
    pub fn builder(
        advance: SpecialAdvance,
        required_advance: Advance,
        name: &str,
        description: &str,
    ) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(advance, required_advance, name.to_string(), description.to_string())
    }

    fn new(
        advance: SpecialAdvance,
        name: String,
        description: String,
        required_advance: Advance,
        listeners: AbilityListeners,
    ) -> Self {
        Self {
            advance,
            name,
            description,
            required_advance,
            listeners,
        }
    }
}

pub struct SpecialAdvanceBuilder {
    advance: SpecialAdvance,
    name: String,
    description: String,
    required_advance: Advance,
    builder: AbilityInitializerBuilder,
}

impl SpecialAdvanceBuilder {
    fn new(advance: SpecialAdvance, required_advance: Advance, name: String, description: String) -> Self {
        Self {
            advance,
            name,
            description,
            required_advance,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn build(self) -> SpecialAdvanceInfo {
        SpecialAdvanceInfo::new(
            self.advance,
            self.name,
            self.description,
            self.required_advance,
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
}
