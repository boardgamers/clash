use serde::{Deserialize, Serialize};

use crate::{
    game::Game, hexagon::Position, playing_actions::ActionType, resource_pile::ResourcePile,
};

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
                let wonder_cards_index = game.players[player_index]
                    .wonder_cards
                    .iter()
                    .position(|wonder_card| wonder_card.name == wonder)
                    .expect("Illegal action");
                let wonder = game.players[player_index]
                    .wonder_cards
                    .remove(wonder_cards_index);
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

    pub fn undo(self, game: &mut Game, player_index: usize) {
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder: _,
                payment,
            } => {
                game.players[player_index].gain_resources(payment);
                let wonder = game.undo_build_wonder(&city_position, player_index);
                game.players[player_index].wonder_cards.push(wonder);
            }
        }
    }
}

impl CustomActionType {
    pub fn action_type(&self) -> ActionType {
        match self {
            CustomActionType::ConstructWonder => ActionType::default(),
        }
    }
}
