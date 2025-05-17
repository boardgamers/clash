use enumset::EnumSetType;
use serde::{Deserialize, Serialize};
use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::events::EventOrigin;

#[derive(EnumSetType, Serialize, Deserialize, Debug, Ord, PartialOrd, Hash)]
pub enum SpecialAdvance {
    // Maya
    Terrace,

    // Rome



}

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
        name: &str,
        required_advance: Advance,
    ) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(advance, name.to_string(), required_advance)
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
    fn new(advance: SpecialAdvance, name: String, required_advance: Advance) -> Self {
        Self {
            advance,
            name,
            description: String::new(),
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
