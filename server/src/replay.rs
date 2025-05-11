use crate::action::{Action, execute_action};
use crate::consts::NON_HUMAN_PLAYERS;
use crate::game::{Game, GameOptions};
use crate::game_setup::setup_game;
use serde::{Deserialize, Serialize};
use std::mem;

///
/// Minimal data for replay - try to avoid breaking changes as much as possible
///
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ReplayGameData {
    #[serde(default)]
    options: GameOptions,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    seed: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_log: Vec<ReplayActionLogAge>,
    players: Vec<ReplayPlayerData>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayActionLogAge {
    pub rounds: Vec<ReplayActionLogRound>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayActionLogRound {
    pub players: Vec<ReplayActionLogPlayer>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayActionLogPlayer {
    pub index: usize,
    pub items: Vec<ReplayActionLogItem>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayActionLogItem {
    pub action: Action,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ReplayPlayerData {}

#[must_use]
pub fn replay(mut data: ReplayGameData, to: Option<usize>) -> Game {
    let log = linear_action_log(mem::take(&mut data.action_log));
    let to = to.unwrap_or(log.len() - 1);
    let mut game = setup_game(
        data.players.len() - NON_HUMAN_PLAYERS,
        data.seed,
        true,
        data.options,
    );

    for (i, a) in log.into_iter().enumerate() {
        if i > to {
            break;
        }
        let player_index = game.active_player();
        game = execute_action(game, a, player_index);
    }
    game
}

pub(crate) fn linear_action_log(log: Vec<ReplayActionLogAge>) -> Vec<Action> {
    log.into_iter()
        .flat_map(|age| {
            age.rounds.into_iter().flat_map(|round| {
                round
                    .players
                    .into_iter()
                    .flat_map(|player| player.items.into_iter().map(|item| item.action))
            })
        })
        .collect()
}
