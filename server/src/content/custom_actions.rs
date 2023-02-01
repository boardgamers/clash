use serde::{Deserialize, Serialize};

use crate::{
    game::Game, hexagon::Position, playing_actions::ActionType, resource_pile::ResourcePile,
};

use super::wonders;

#[derive(Serialize, Deserialize)]
pub enum CustomAction {
    ConstructWonder {
        city_position: Position,
        wonder: String,
        payment: ResourcePile,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CustomActionType {
    ConstructWonder,
}

impl CustomAction {
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder,
                payment,
            } => {
                let wonder = wonders::get_wonder_by_name(&wonder)
                    .expect("construct wonder data should include a valid wonder name");
                let city = game.players[player_index]
                    .get_city(&city_position)
                    .expect("player should have city");
                if !city.can_build_wonder(&wonder, &game.players[player_index])
                    || !payment.can_afford(&wonder.cost)
                {
                    panic!("Illegal action");
                }
                game.players[player_index].loose_resources(payment);
                game.build_wonder(wonder, &city_position, player_index);
            }
        }
    }

    pub fn custom_action_type(&self) -> CustomActionType {
        match self {
            CustomAction::ConstructWonder { .. } => CustomActionType::ConstructWonder,
        }
    }
}

impl CustomActionType {
    pub fn action_type(&self) -> ActionType {
        match self {
            _ => ActionType::default(),
        }
    }
}
