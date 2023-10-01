use std::{cmp::Ordering::*, mem};

use crate::{
    action::Action,
    game::{Game, GameState::*, Messages},
    log::LogSliceOptions,
};

// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[must_use]
pub fn init(player_amount: usize, seed: String) -> Game {
    Game::new(player_amount, seed, true)
}

#[must_use]
pub fn execute_action(mut game: Game, action: Action, player_index: usize) -> Game {
    game.execute_action(action, player_index);
    game
}

#[must_use]
pub fn ended(game: &Game) -> bool {
    matches!(game.state, Finished)
}

#[must_use]
pub fn scores(game: &Game) -> Vec<f32> {
    let mut scores: Vec<f32> = Vec::new();
    for player in &game.players {
        scores.push(player.victory_points());
    }
    scores
}

#[must_use]
pub fn drop_player(mut game: Game, player_index: usize) -> Game {
    game.drop_player(player_index);
    game
}

#[must_use]
pub fn current_player(game: &Game) -> Option<usize> {
    match game.state {
        Finished => None,
        _ => Some(game.active_player()),
    }
}

#[must_use]
pub fn log_length(game: &Game) -> usize {
    game.log.len()
}

#[must_use]
pub fn log_slice(game: &Game, options: &LogSliceOptions) -> Vec<String> {
    match options.end {
        Some(end) => &game.log[options.start..=end],
        None => &game.log[options.start..],
    }
    .to_vec()
}

#[must_use]
pub fn set_player_name(mut game: Game, player_index: usize, name: String) -> Game {
    game.players[player_index].set_name(name);
    game
}

#[must_use]
pub fn rankings(game: &Game) -> Vec<u32> {
    let mut rankings = Vec::new();
    for player in &game.players {
        let mut rank = 1;
        for other in &game.players {
            if other.compare_score(player) == Greater {
                rank += 1;
            }
        }
        rankings.push(rank);
    }
    rankings
}

#[must_use]
pub fn round(game: &Game) -> Option<u32> {
    match game.state {
        Playing => Some((game.age - 1) * 3 + game.round),
        _ => None,
    }
}

#[must_use]
pub fn civilizations(game: Game) -> Vec<String> {
    game.players
        .into_iter()
        .map(|player| player.civilization.name)
        .collect()
}

#[must_use]
pub fn strip_secret(mut game: Game, player_index: Option<usize>) -> Game {
    game.dice_roll_outcomes = Vec::new();
    game.wonders_left = Vec::new();
    for (i, player) in game.players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret();
        }
    }
    game
}

#[must_use]
pub fn messages(mut game: Game) -> Messages {
    let messages = mem::take(&mut game.messages);
    Messages::new(messages, game.data())
}
