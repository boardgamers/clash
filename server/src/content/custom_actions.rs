use serde::{Deserialize, Serialize};

use crate::{
    game::Game, playing_actions::ActionType, position::Position, resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CustomAction {
    ConstructWonder {
        city_position: Position,
        wonder: String,
        payment: ResourcePile,
    },
    WhipWorkers,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CustomActionType {
    ConstructWonder,
    WhipWorkers,
}

impl CustomAction {
    ///
    ///
    /// # Panics
    ///
    /// Panics if action is illegal
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder,
                payment,
            } => {
                let wonder_cards_index = game.players[player_index]
                    .wonder_cards
                    .iter()
                    .position(|wonder_card| wonder_card.name == wonder)
                    .expect("Illegal action");
                let wonder = game.players[player_index]
                    .wonder_cards
                    .remove(wonder_cards_index);
                let city = game.players[player_index]
                    .get_city(city_position)
                    .expect("player should have city");
                if !city.can_build_wonder(&wonder, &game.players[player_index], game)
                    || !payment.can_afford(&wonder.cost)
                {
                    panic!("Illegal action");
                }
                game.players[player_index].loose_resources(payment);

                game.build_wonder(wonder, city_position, player_index);
            }
        }
    }

    #[must_use]
    pub fn custom_action_type(&self) -> CustomActionType {
        match self {
            CustomAction::ConstructWonder { .. } => CustomActionType::ConstructWonder,
            CustomAction::WhipWorkers => CustomActionType::WhipWorkers,
        }
    }

    pub fn undo(self, game: &mut Game, player_index: usize) {
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder: _,
                payment,
            } => {
                game.players[player_index].gain_resources(payment);
                let wonder = game.undo_build_wonder(city_position, player_index);
                game.players[player_index].wonder_cards.push(wonder);
            }
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, player_name: &str) -> String {
        match self {
            CustomAction::ConstructWonder { city_position, wonder, payment } => format!("{player_name} paid {payment} to construct the {wonder} wonder in the city at {city_position}"),
        }
    }
}

impl CustomActionType {
    #[must_use]
    pub fn action_type(&self) -> ActionType {
        match self {
            CustomActionType::ConstructWonder => ActionType::default(),
            CustomActionType::WhipWorkers => ActionType::free_and_once_per_turn(),
        }
    }
}
