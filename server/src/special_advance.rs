use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::advance::Advance;
use crate::events::EventOrigin;

pub struct SpecialAdvance {
    pub advance: Advance,
    pub name: String,
    pub description: String,
    pub required_advance: Advance,
    pub listeners: AbilityListeners,
}

impl SpecialAdvance {
    #[must_use]
    pub fn builder(
        advance: Advance,
        name: &str,
        required_advance: Advance,
    ) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(advance, name.to_string(), required_advance)
    }

    fn new(
        advance: Advance,
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
    advance: Advance,
    name: String,
    descriptions: Vec<String>,
    required_advance: Advance,
    builder: AbilityInitializerBuilder,
}

impl SpecialAdvanceBuilder {
    fn new(advance: Advance, name: String, required_advance: Advance) -> Self {
        Self {
            advance,
            name,
            descriptions: Vec::new(),
            required_advance,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn build(self) -> SpecialAdvance {
        SpecialAdvance::new(
            self.advance,
            self.name,
            String::from("✦ ") + &self.descriptions.join("\n✦ "),
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
