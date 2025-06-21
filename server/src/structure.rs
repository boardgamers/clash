use crate::city_pieces::Building;
use crate::events::EventPlayer;
use crate::game::Game;
use crate::log::{ActionLogBalance, ActionLogEntry, add_action_log_item};
use crate::player::Player;
use crate::position::Position;
use crate::wonder::Wonder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Structure {
    CityCenter,
    Building(Building),
    Wonder(Wonder),
}

impl Structure {
    #[must_use]
    pub fn is_available(&self, player: &Player, game: &Game) -> bool {
        match self {
            Structure::CityCenter => player.is_city_available(),
            Structure::Building(b) => player.is_building_available(*b, game),
            Structure::Wonder(_) => false,
        }
    }
}

pub(crate) fn log_gain_structure(
    game: &mut Game,
    player: &EventPlayer,
    structure: Structure,
    position: Position,
) {
    player.log(
        game,
        &format!("Gain city {}", position),
    );
    log_structure(game, player, structure, ActionLogBalance::Gain, position);
}

pub(crate) fn log_lose_structure(
    game: &mut Game,
    player: &EventPlayer,
    structure: Structure,
    position: Position,
) {
    player.log(
        game,
      &format!("Lose city {}", position),
    );
    log_structure(game, player, structure, ActionLogBalance::Loss, position);
}

fn log_structure(
    game: &mut Game,
    player: &EventPlayer,
    structure: Structure,
    balance: ActionLogBalance,
    position: Position,
) {
    add_action_log_item(
        game,
        player.index,
        ActionLogEntry::structure(structure, balance, position),
        player.origin.clone(),
        vec![],
    );
}
