use serde::{Deserialize, Serialize};
use std::{cmp::Ordering::*, mem};

use crate::game::Action;
use crate::game::Game;
use crate::game::GameData;
use crate::game::GameState::*;
use crate::game::LogItem;

#[derive(Serialize, Deserialize)]
pub struct LogSliceOptions {
    player: Option<usize>,
    start: usize,
    end: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Log {
    items: Vec<LogItem>,
}

impl Log {
    pub fn new(items: Vec<LogItem>) -> Self {
        Self { items }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Messages{messages: Vec<String>, data: GameData}

impl Messages {
    pub fn new(messages: Vec<String>, data: GameData) -> Self { Self { messages, data } }
}

// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

pub fn init(
    player_amount: usize,
    seed: String,
) -> Game {
    Game::new(player_amount, seed)
}

pub fn execute_action(mut game: Game, action: Action, player_index: usize) -> Game {
    game.execute_action(action, player_index);
    game
}

pub fn ended(game: Game) -> bool {
    matches!(game.state, Finished)
}

pub fn scores(game: Game) -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new();
    for player in game.players.iter() {
        scores.push(player.victory_points());
    }
    scores
}

pub fn drop_player(mut game: Game, player_index: usize) -> Game {
    game.drop_player(player_index);
    game
}

pub fn current_player(game: Game) -> Option<usize> {
    match game.state {
        Finished => None,
        _ => Some(game.current_player_index),
    }
}

pub fn log_length(game: Game) -> usize {
    game.log.len()
}

pub fn log_slice(game: Game, options: LogSliceOptions) -> Log {
    let log_slice = match options.end {
        Some(end) => &game.log[options.start..=end],
        None => &game.log[options.start..],
    }
    .to_vec();
    Log::new(log_slice)
}

pub fn set_player_name(mut game: Game, player_index: usize, name: String) -> Game {
    game.players[player_index].set_name(name);
    game
}

pub fn rankings(game: Game) -> Vec<u32> {
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

pub fn round(game: Game) -> Option<u32> {
    match game.state {
        Playing => Some((game.age - 1) * 3 + game.round),
        _ => None,
    }
}

pub fn civilizations(game: Game) -> Vec<String> {
    game.players
        .into_iter()
        .map(|player| player.civilization.name)
        .collect()
}

pub fn strip_secret(mut game: Game, player_index: Option<usize>) -> Game {
    let player_index = player_index;
    game.dice_roll_outcomes = Vec::new();
    game.wonders_left = Vec::new();
    for (i, player) in game.players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret()
        }
    }
    game
}

pub fn messages(mut game: Game) -> Messages {
    let messages = mem::take(&mut game.messages);
    Messages::new(messages, game.data())
}
