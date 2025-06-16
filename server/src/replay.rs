use crate::action::{Action, try_execute_action};
use crate::consts::NON_HUMAN_PLAYERS;
use crate::game::{Game, GameContext, GameOptions};
use crate::game_setup::{GameSetupBuilder, setup_game};
use itertools::Itertools;
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dropped_players: Vec<usize>,
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
    pub actions: Vec<ReplayActionLogAction>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayActionLogAction {
    pub action: Action,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ReplayPlayerData {
    id: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    civilization: String,
}

/// replay is used to store the game data for replay
///
/// # Panics
///
/// Panics if the game data cannot be replayed
#[must_use]
pub fn replay(mut data: ReplayGameData, to: Option<usize>) -> Game {
    let log = linear_action_log(mem::take(&mut data.action_log));
    let to = to.unwrap_or(log.len() - 1);
    let mut game = setup_game(
        &GameSetupBuilder::new(data.players.len() - NON_HUMAN_PLAYERS)
            .seed(data.seed)
            .options(data.options)
            .civilizations(
                data.players
                    .iter()
                    .map(|player| player.civilization.clone())
                    .collect_vec(),
            )
            .build(),
    );
    game.dropped_players = data.dropped_players;
    for player in &data.players {
        if let Some(name) = &player.name {
            game.players[player.id].set_name(name.clone());
        }
    }
    game.context = GameContext::Replay;

    for (i, (id, a)) in log.into_iter().enumerate() {
        if i > to {
            break;
        }
        let player_index = game.active_player();
        match try_execute_action(game, a.clone(), player_index) {
            Ok(g) => game = g,
            Err(e) => {
                panic!("Failed to execute action {id}, {a:?}: {e}");
            }
        }
    }
    game
}

pub(crate) fn linear_action_log(log: Vec<ReplayActionLogAge>) -> Vec<(String, Action)> {
    log.into_iter()
        .enumerate()
        .flat_map(move |(age_num, age)| {
            age.rounds
                .into_iter()
                .enumerate()
                .flat_map(move |(round_num, round)| {
                    round.players.into_iter().flat_map(move |player| {
                        player.actions.into_iter().map(move |item| {
                            (format!("{age_num}{round_num}{}", player.index), item.action)
                        })
                    })
                })
        })
        .collect()
}
