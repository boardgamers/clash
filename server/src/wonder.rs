use crate::ability_initializer::EventOrigin;
use crate::payment::PaymentOptions;
use crate::{
    ability_initializer::{self, AbilityInitializer, AbilityInitializerSetup},
    game::Game,
    position::Position,
};

type PlacementChecker = Box<dyn Fn(Position, &Game) -> bool>;

pub struct Wonder {
    pub name: String,
    pub description: String,
    pub cost: PaymentOptions,
    pub required_advances: Vec<String>,
    pub placement_requirement: Option<PlacementChecker>,
    pub player_initializer: AbilityInitializer,
    pub player_deinitializer: AbilityInitializer,
    pub player_one_time_initializer: AbilityInitializer,
    pub player_undo_deinitializer: AbilityInitializer,
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
    player_initializers: Vec<AbilityInitializer>,
    player_deinitializers: Vec<AbilityInitializer>,
    player_one_time_initializers: Vec<AbilityInitializer>,
    player_undo_deinitializers: Vec<AbilityInitializer>,
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
            player_initializers: Vec::new(),
            player_deinitializers: Vec::new(),
            player_one_time_initializers: Vec::new(),
            player_undo_deinitializers: Vec::new(),
        }
    }

    pub fn add_description(mut self, description: &str) -> Self {
        self.descriptions.push(description.to_string());
        self
    }

    pub fn placement_requirement(&mut self, placement_requirement: PlacementChecker) -> &mut Self {
        self.placement_requirement = Some(placement_requirement);
        self
    }

    pub fn build(self) -> Wonder {
        let player_initializer =
            ability_initializer::join_ability_initializers(self.player_initializers);
        let player_deinitializer =
            ability_initializer::join_ability_initializers(self.player_deinitializers);
        let player_one_time_initializer =
            ability_initializer::join_ability_initializers(self.player_one_time_initializers);
        let player_undo_deinitializer =
            ability_initializer::join_ability_initializers(self.player_undo_deinitializers);
        Wonder {
            name: self.name,
            description: String::from("✦ ") + &self.descriptions.join("\n✦ "),
            cost: self.cost,
            required_advances: self.required_advances,
            placement_requirement: self.placement_requirement,
            player_initializer,
            player_deinitializer,
            player_one_time_initializer,
            player_undo_deinitializer,
        }
    }
}

impl AbilityInitializerSetup for WonderBuilder {
    fn add_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_initializers.push(Box::new(initializer));
        self
    }

    fn add_ability_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_deinitializers.push(Box::new(deinitializer));
        self
    }

    fn add_one_time_ability_initializer<F>(mut self, initializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_one_time_initializers
            .push(Box::new(initializer));
        self
    }

    fn add_ability_undo_deinitializer<F>(mut self, deinitializer: F) -> Self
    where
        F: Fn(&mut Game, usize) + 'static,
    {
        self.player_undo_deinitializers
            .push(Box::new(deinitializer));
        self
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::Wonder(self.name.clone())
    }
}
