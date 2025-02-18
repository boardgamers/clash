use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::events::EventOrigin;

pub struct SpecialAdvance {
    pub name: String,
    pub description: String,
    pub required_advance: String,
    pub listeners: AbilityListeners,
}

impl SpecialAdvance {
    #[must_use]
    pub fn builder(name: &str, required_advance: &str) -> SpecialAdvanceBuilder {
        SpecialAdvanceBuilder::new(name.to_string(), required_advance.to_string())
    }

    fn new(
        name: String,
        description: String,
        required_advance: String,
        listeners: AbilityListeners,
    ) -> Self {
        Self {
            name,
            description,
            required_advance,
            listeners,
        }
    }
}

pub struct SpecialAdvanceBuilder {
    name: String,
    descriptions: Vec<String>,
    required_advance: String,
    builder: AbilityInitializerBuilder,
}

impl SpecialAdvanceBuilder {
    fn new(name: String, required_advance: String) -> Self {
        Self {
            name,
            descriptions: Vec::new(),
            required_advance,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn build(self) -> SpecialAdvance {
        SpecialAdvance::new(
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
        EventOrigin::SpecialAdvance(self.name.clone())
    }
}
