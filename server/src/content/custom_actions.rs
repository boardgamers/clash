use serde::{Deserialize, Serialize};

use crate::{
    game::Game, playing_actions::ActionType, position::Position, resource_pile::ResourcePile,
};
use crate::content::wonders::construct_wonder;

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
            } => construct_wonder(game, player_index, city_position, wonder, payment),
            CustomAction::WhipWorkers => {
                game.actions_left += 1;
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
            },
            CustomAction::WhipWorkers => game.actions_left -= 1,
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, player_name: &str) -> String {
        match self {
            CustomAction::ConstructWonder { city_position, wonder, payment } => format!("{player_name} paid {payment} to construct the {wonder} wonder in the city at {city_position}"),
            CustomAction::WhipWorkers => format!("{player_name} whipped workers"),
        }
    }
}

impl CustomActionType {
    #[must_use]
    pub fn action_type(&self) -> ActionType {
        match self {
            CustomActionType::ConstructWonder => ActionType::default(),
            CustomActionType::WhipWorkers => ActionType::free_and_once_per_turn(ResourcePile::mood_tokens(2)),
        }
    }
}
