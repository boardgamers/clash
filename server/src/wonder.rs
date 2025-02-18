use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::events::EventOrigin;
use crate::payment::PaymentOptions;
use crate::{ability_initializer::AbilityInitializerSetup, game::Game, position::Position};

type PlacementChecker = Box<dyn Fn(Position, &Game) -> bool>;

pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advances: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub listeners: AbilityListeners,
}

impl Wonder {
    pub fn builder(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<&str>,
    ) -> WonderBuilder {
        WonderBuilder::new(
            name,
            description,
            cost,
            required_advances
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        )
    }
}

pub struct WonderBuilder {
    name: String,
    descriptions: Vec<String>,
    cost: PaymentOptions,
    required_advances: Vec<String>,
    placement_requirement: Option<PlacementChecker>,
    builder: AbilityInitializerBuilder,
}

impl WonderBuilder {
    fn new(
        name: &str,
        description: &str,
        cost: PaymentOptions,
        required_advances: Vec<String>,
    ) -> Self {
        Self {
            name: name.to_string(),
            descriptions: vec![description.to_string()],
            cost,
            required_advances,
            placement_requirement: None,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    pub fn placement_requirement(&mut self, placement_requirement: PlacementChecker) -> &mut Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    pub fn build(self) -> Wonder {
        Wonder {
            name: self.name,
            description: String::from("✦ ") + &self.descriptions.join("\n✦ "),
            cost: self.cost,
            required_advances: self.required_advances,
            placement_requirement: self.placement_requirement,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for WonderBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Wonder(self.name.clone())
    }
}
