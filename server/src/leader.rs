use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::events::EventOrigin;

pub struct Leader {
    pub name: String,
    pub first_ability: String,
    pub first_ability_description: String,
    pub second_ability: String,
    pub second_ability_description: String,
    pub listeners: AbilityListeners,
}

impl Leader {
    #[must_use]
    pub fn builder(
        name: &str,
        first_ability: &str,
        first_ability_description: &str,
        second_ability: &str,
        second_ability_description: &str,
    ) -> LeaderBuilder {
        LeaderBuilder::new(
            name.to_string(),
            first_ability.to_string(),
            first_ability_description.to_string(),
            second_ability.to_string(),
            second_ability_description.to_string(),
        )
    }
}

pub struct LeaderBuilder {
    name: String,
    first_ability: String,
    first_ability_description: String,
    second_ability: String,
    second_ability_description: String,
    builder: AbilityInitializerBuilder,
}

impl LeaderBuilder {
    fn new(
        name: String,
        first_ability: String,
        first_ability_description: String,
        second_ability: String,
        second_ability_description: String,
    ) -> Self {
        Self {
            name,
            first_ability,
            first_ability_description,
            second_ability,
            second_ability_description,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> Leader {
        Leader {
            name: self.name,
            first_ability: self.first_ability,
            first_ability_description: self.first_ability_description,
            second_ability: self.second_ability,
            second_ability_description: self.second_ability_description,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for LeaderBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Leader(self.name.clone())
    }
}
