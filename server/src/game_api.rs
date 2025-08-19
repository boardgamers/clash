use super::player::Player;
use crate::action::execute_action;
use crate::card::{HandCard, HandCardLocation};
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::game::GameOptions;
use crate::game_setup::{GameSetupBuilder, setup_game};
use crate::log::{ActionLogAction, ActionLogEntry, linear_action_log};
use crate::utils::Shuffle;
use crate::victory_points::compare_score;
use crate::wonder::Wonder;
use crate::{
    action::Action,
    game::{Game, GameState::*},
    log::LogSliceOptions,
    utils::Rng,
};
use std::cmp::Ordering::*;
// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[must_use]
pub fn init(player_amount: usize, seed: String, options: GameOptions) -> Game {
    setup_game(
        &GameSetupBuilder::new(player_amount)
            .seed(seed)
            .options(options)
            .build(),
    )
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
    linear_action_log(game).len()
}

#[must_use]
pub fn log_slice(game: &Game, options: &LogSliceOptions) -> Vec<Action> {
    let l = linear_action_log(game);
    match options.end {
        Some(end) => &l[options.start..=end],
        None => &l[options.start..],
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
            if compare_score(other, player, game) == Greater {
                rank += 1;
            }
        }
        rankings.push(rank);
    }
    rankings
}

#[must_use]
pub fn round(game: &Game) -> u32 {
    // idea: you can easily see that "12" is age 1, round 2
    // round 4 is status phase
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
        if let PermanentEffect::GreatSeer(g) = e
            && player_index != Some(g.player)
        {
            // player shouldn't see other player's great seer
            g.strip_secret();
        }
    }
    game.incidents_left.shuffle(&mut game.rng);
    game.wonders_left.shuffle(&mut game.rng);
    game.action_cards_left.shuffle(&mut game.rng);
    game.objective_cards_left.shuffle(&mut game.rng);
    game.seed = String::new();
    game.rng = Rng::default();
    for (i, player) in game.players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret();
        }
    }
    game.map.strip_secret();
    strip_events(&mut game, player_index);
    strip_log(&mut game, player_index);

    game
}

fn strip_log(game: &mut Game, player_index: Option<usize>) {
    for age in &mut game.log {
        for round in &mut age.rounds {
            for player in &mut round.turns {
                for action in &mut player.actions {
                    strip_action(action, player_index);
                }
            }
        }
    }
}

fn strip_action(action: &mut ActionLogAction, player_index: Option<usize>) {
    // undo has secret information, like gained action cards
    action.undo.clear();

    if let Action::Response(EventResponse::SelectHandCards(c)) = &mut action.action {
        // player shouldn't see other player's hand cards
        c.clear();
    }

    for item in &mut action.items {
        if let ActionLogEntry::HandCard { card, from, to } = &mut item.entry
            && is_visible_card_info(player_index, from, to)
        {
            match &card {
                HandCard::ActionCard(_) => *card = HandCard::ActionCard(0),
                HandCard::ObjectiveCard(_) => *card = HandCard::ObjectiveCard(0),
                HandCard::Wonder(_) => *card = HandCard::Wonder(Wonder::Hidden),
            }
        }
    }
}

fn is_visible_card_info(
    player_index: Option<usize>,
    from: &HandCardLocation,
    to: &HandCardLocation,
) -> bool {
    from.player() == player_index
        || to.player() == player_index
        || from.is_public()
        || to.is_public()
}

fn strip_events(game: &mut Game, player_index: Option<usize>) {
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
                if let Some(handler) = &mut s.player.handler
                    && let PersistentEventRequest::SelectHandCards(c) = &mut handler.request
                {
                    c.description = "Complete an objective card".to_string();
                }
            }
            _ => {}
        }
        let current_event_player = &mut s.player;
        if player_index != Some(current_event_player.index)
            && let Some(handler) = &mut current_event_player.handler
        {
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
