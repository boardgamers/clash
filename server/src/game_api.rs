use std::{cmp::Ordering::*, mem};

use serde::{Deserialize, Serialize};

use crate::action::PlayingAction;
use crate::game::Game;
use crate::game::GameState::*;

#[derive(Serialize, Deserialize)]
pub struct UserAction {
    pub action: PlayingAction,
    pub specification: Option<String>,
}

impl UserAction {
    pub fn new(action: PlayingAction, specification: Option<String>) -> Self {
        Self {
            action,
            specification,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LogSliceOptions {
    player: Option<usize>,
    start: usize,
    end: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Log {
    items: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerMetaData {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Messages(Vec<String>, String);

// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[export_name = "init"]
pub async extern "C" fn init(
    players: usize,
    _expansions: String,
    _options: String,
    seed: String,
    _creator: String,
) -> String {
    Game::new(players, seed).json()
}

#[export_name = "move"]
pub extern "C" fn execute_action(game: String, r#move: String, player: usize) -> String {
    let mut game = Game::from_json(&game);
    let user_action =
        serde_json::from_str(&r#move).expect("API call should receive valid move json");
    game.log.push(r#move);
    game.execute_playing_action(user_action, player);
    game.json()
}

#[export_name = "ended"]
pub extern "C" fn ended(game: String) -> bool {
    let game = Game::from_json(&game);
    matches!(game.state, Finished)
}

#[export_name = "scores"]
pub extern "C" fn scores(game: String) -> Vec<f32> {
    let game = Game::from_json(&game);
    let mut scores: Vec<f32> = Vec::new();
    for player in game.players.iter() {
        scores.push(player.victory_points());
    }
    scores
}

#[export_name = "dropPlayer"]
pub async extern "C" fn drop_player(game: String, player: usize) -> String {
    let mut game = Game::from_json(&game);
    game.players.remove(player);
    game.json()
}

#[export_name = "currentPlayer"]
pub async extern "C" fn current_player(game: String) -> Option<usize> {
    let game = Game::from_json(&game);
    match game.state {
        Finished => None,
        _ => Some(game.current_player),
    }
}

#[export_name = "logLength"]
pub async extern "C" fn log_length(game: String) -> usize {
    let game = Game::from_json(&game);
    game.log.len()
}

#[export_name = "logSlice"]
pub async extern "C" fn log_slice(game: String, options: String) -> String {
    let game = Game::from_json(&game);
    let options =
        serde_json::from_str::<LogSliceOptions>(&options).expect("options should be serializable");
    let log_slice = match options.end {
        Some(end) => &game.log[options.start..=end],
        None => &game.log[options.start..],
    }
    .to_vec();
    serde_json::to_string(&log_slice).expect("log slice should be serializable")
}

#[export_name = "setPlayerMetaData"]
pub extern "C" fn set_player_meta_data(game: String, player: usize, meta_data: String) -> String {
    let name = serde_json::from_str::<PlayerMetaData>(&meta_data)
        .expect("")
        .name;
    let mut game = Game::from_json(&game);
    game.players[player].set_name(name);
    game.json()
}

#[export_name = "rankings"]
pub extern "C" fn rankings(game: String) -> Vec<u32> {
    let game = Game::from_json(&game);
    let mut rankings = Vec::new();
    for player in game.players.iter() {
        let mut rank = 1;
        for other in game.players.iter() {
            if other.compare_score(player) == Greater {
                rank += 1;
            }
        }
        rankings.push(rank);
    }
    rankings
}

#[export_name = "round"]
pub extern "C" fn round(game: String) -> Option<u32> {
    let game = Game::from_json(&game);
    match game.state {
        Playing => Some((game.age - 1) * 3 + game.round),
        _ => None,
    }
}

#[export_name = "factions"]
pub extern "C" fn civilizations(game: String) -> Vec<String> {
    let game = Game::from_json(&game);
    game.players
        .into_iter()
        .map(|player| player.civilization.name)
        .collect()
}

#[export_name = "stripSecret"]
pub extern "C" fn strip_secret(game: String, player: Option<usize>) -> String {
    let player_index = player;
    let mut game = Game::from_json(&game);
    game.dice_roll_outcomes = Vec::new();
    for (i, player) in game.players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret()
        }
    }
    game.json()
}

#[export_name = "messages"]
pub extern "C" fn messages(game: String) -> String {
    let mut game = Game::from_json(&game);
    let messages = mem::take(&mut game.messages);
    serde_json::to_string(&Messages(messages, game.json()))
        .expect("Messages should be serializable")
}
