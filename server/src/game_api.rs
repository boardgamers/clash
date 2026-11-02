use super::player::Player;
use crate::action::execute_action;
use crate::card::{HandCard, HandCardLocation};
use crate::combat_listeners::CombatEventPhase;
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
use std::mem;
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
    let log = linear_action_log(game);
    // BGS requests the post-move delta using the pre-move length, which can be
    // beyond the end after undo. Only return entries still in the active log.
    let end = options
        .end
        .map_or(log.len(), |end| end.saturating_add(1).min(log.len()));
    log.get(options.start..end).unwrap_or_default().to_vec()
}

/// BGS consumes `text` for its ongoing-game list; replay keeps using saved actions.
#[derive(serde::Serialize)]
pub struct PlatformLogEntry<'a> {
    pub action: &'a Action,
    pub player: usize,
    pub text: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
}

#[must_use]
pub fn platform_log_slice<'a>(
    game: &'a Game,
    options: &LogSliceOptions,
) -> Vec<PlatformLogEntry<'a>> {
    let log = crate::log::linear_action_entries(game);
    let end = options
        .end
        .map_or(log.len(), |end| end.saturating_add(1).min(log.len()));
    log.get(options.start..end)
        .unwrap_or_default()
        .iter()
        .map(|entry| PlatformLogEntry {
            action: &entry.action,
            player: entry.player,
            kind: (matches!(entry.action, Action::StartTurn) && entry.items.is_empty())
                .then_some("phase"),
            text: crate::move_summary::summary(game, entry),
        })
        .collect()
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
            if other.is_human() && compare_score(other, player, game) == Greater {
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
    if game
        .action_plan
        .as_ref()
        .is_some_and(|p| Some(p.player) != player_index)
    {
        game.action_plan = None;
    }
    game.early_choices
        .retain(|c| Some(c.player) == player_index);
    if let Some(draft) = &mut game.civilization_draft {
        for i in 0..draft.offers.len() {
            if player_index != Some(i) {
                draft.offers[i].clear();
                draft.choices[i] = None;
            }
        }
    }
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
    // Publish the stable history identity before the first move, so a filtered
    // client can predict that move without needing the secret setup seed.
    game.board_history.ensure_id(&game.seed);
    game.seed = String::new();
    game.rng = Rng::default();
    let mut players = mem::take(&mut game.players);
    for (i, player) in players.iter_mut().enumerate() {
        if player_index != Some(i) {
            player.strip_secret(&game);
        }
    }
    game.players = players;
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
            && !is_visible_card_info(player_index, from, to)
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
    player_index.is_some_and(|player| from.player() == Some(player) || to.player() == Some(player))
        || from.is_public()
        || to.is_public()
}

fn strip_events(game: &mut Game, player_index: Option<usize>) {
    let mut pending_tactics = Vec::new();
    for s in &mut game.events {
        match &mut s.event_type {
            PersistentEventType::CustomAction(a)
                if a.action.action
                    == crate::content::custom_actions::CustomActionType::Calendar
                    && Some(s.player.index) != player_index =>
            {
                if let Some(handler) = &mut s.player.handler {
                    if let PersistentEventRequest::Payment(requests) = &mut handler.request {
                        for request in requests {
                            request.name = "Calendar · Inspecting the next event".into();
                        }
                    }
                }
            }
            PersistentEventType::CombatRoundStart(r)
                if matches!(
                    r.phase,
                    CombatEventPhase::AllowTacticsCard
                        | CombatEventPhase::Loyalty
                        | CombatEventPhase::Default
                ) =>
            {
                // Committing immediately adds the card to the discard pile. Save
                // the real IDs before replacing them with the presence sentinel.
                for (card, participant) in [
                    (r.attacker_strength.tactics_card, r.combat.attacker()),
                    (r.defender_strength.tactics_card, r.combat.defender()),
                ] {
                    if let Some(card) = card.filter(|id| *id != 0) {
                        pending_tactics.push((card, participant));
                    }
                }
                // Either side may commit first (Loyalty can supply the defender).
                // Keep presence for eligibility/status, but never disclose the ID
                // before reveal. Owners still see their own card in the transfer log.
                r.attacker_strength.tactics_card = r.attacker_strength.tactics_card.map(|_| 0);
                r.defender_strength.tactics_card = r.defender_strength.tactics_card.map(|_| 0);
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
    pending_tactics.sort_unstable();
    pending_tactics.dedup();
    for (card, participant) in pending_tactics {
        // The hand owner may be the Loyalty controller, not the combatant.
        // Match discard_card's insertion end and mask only this commitment:
        // an earlier, publicly discarded copy of the same ID can remain here.
        let owner = crate::content::civilizations::celts::delegated_player(game, participant);
        let discarded = &mut game.action_cards_discarded;
        let position = if game.players[owner]
            .wonders_owned
            .contains(Wonder::GreatMausoleum)
        {
            discarded.iter().position(|id| *id == card)
        } else {
            discarded.iter().rposition(|id| *id == card)
        };
        if let Some(position) = position {
            discarded[position] = 0;
        }
    }
}
