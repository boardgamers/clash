use super::player::Player;
use crate::action::execute_action;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game_setup::setup_game;
use crate::log::current_player_turn_log_mut;
use crate::utils::Shuffle;
use crate::{
    action::Action,
    game::{Game, GameState::*},
    log::LogSliceOptions,
    utils::Rng,
};
use std::cmp::Ordering::*;
// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[must_use]
pub fn init(player_amount: usize, seed: String) -> Game {
    setup_game(player_amount, seed, true)
}

#[must_use]
pub fn execute(game: Game, action: Action, player_index: usize) -> Game {
    execute_action(game, action, player_index)
}

#[must_use]
pub fn ended(game: &Game) -> bool {
    matches!(game.state, Finished)
}

#[must_use]
pub fn scores(game: &Game) -> Vec<f32> {
    game.players
        .iter()
        .filter(|p| p.is_human())
        .map(|player| player.victory_points(game))
        .collect()
}

#[must_use]
pub fn drop_player(mut game: Game, player_index: usize) -> Game {
    game.drop_player(player_index);
    game
}

#[must_use]
pub fn log_length(game: &Game) -> usize {
    game.log.len()
}

#[must_use]
pub fn log_slice(game: &Game, options: &LogSliceOptions) -> Vec<Vec<String>> {
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
        if !player.is_human() {
            continue;
        }
        let mut rank = 1;
        for other in &game.players {
            if other.compare_score(player, game) == Greater {
                rank += 1;
            }
        }
        rankings.push(rank);
    }
    rankings
}

#[must_use]
pub fn round(game: &Game) -> u32 {
    (game.age) * 10 + game.round
}

#[must_use]
pub fn civilizations(game: Game) -> Vec<String> {
    game.players
        .into_iter()
        .filter(Player::is_human)
        .map(|player| player.civilization.name)
        .collect()
}

#[must_use]
pub fn strip_secret(mut game: Game, player_index: Option<usize>) -> Game {
    for e in &mut game.permanent_effects {
        if let PermanentEffect::GreatSeer(g) = e {
            if player_index != Some(g.player) {
                // player shouldn't see other player's great seer
                g.strip_secret();
            }
        }
    }
    game.incidents_left.shuffle(&mut game.rng);
    game.wonders_left.shuffle(&mut game.rng);
    game.action_cards_left.shuffle(&mut game.rng);
    game.objective_cards_left.shuffle(&mut game.rng);
    game.rng = Rng::default();
    for (i, player) in game.players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret();
        }
    }
    game.map.strip_secret();
    for s in &mut game.events {
        match &mut s.event_type {
            PersistentEventType::CombatRoundStart(r) => {
                if r.attacker_strength.tactics_card.is_some() {
                    // defender shouldn't see attacker's tactics card
                    r.attacker_strength.tactics_card = Some(0);
                }
            }
            PersistentEventType::SelectObjectives(o) if Some(s.player.index) != player_index => {
                // player shouldn't see other player's objectives
                o.strip_secret();
            }
            _ => {}
        }
        let current_event_player = &mut s.player;
        if player_index != Some(current_event_player.index) {
            if let Some(handler) = &mut current_event_player.handler {
                if let PersistentEventRequest::SelectHandCards(c) = &mut handler.request {
                    // player shouldn't see other player's hand cards
                    c.choices.clear();
                }
                if let Some(EventResponse::SelectHandCards(c)) = &mut handler.response {
                    // player shouldn't see other player's hand cards
                    c.clear();
                }
            }
        }
    }
    let player_log = current_player_turn_log_mut(&mut game);
    if player_index != Some(player_log.index) {
        for l in &mut player_log.items {
            // undo has secret information, like gained action cards
            l.undo.clear();
            if let Action::Response(EventResponse::SelectHandCards(c)) = &mut l.action {
                // player shouldn't see other player's hand cards
                c.clear();
            }
        }
    }

    game
}
