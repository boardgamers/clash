//! Platform clock credits, separate from the turn-local undo cursor and RNG.
//!
//! Only the live, saved-action boundary (`board_history`) calls `execute`.
//! Rules previews, queue validation and historical replay use the lower-level
//! executor and cannot earn credits. Automatic responses and resumed plans pass
//! through the same boundary, attributed to their own actor, not the submitter.
use crate::action::{Action, try_execute_action};
use crate::game::{Game, GameContext, GameState};
use serde::{Deserialize, Deserializer};

/// The protocol requires JavaScript-safe integers, even for very long games.
const MAX_INCREMENT: u64 = (1_u64 << 53) - 1;

/// Cumulative credits in platform seat order, including dropped human players,
/// never NPCs. Missing counters in older saves mean zero, not historical credit.
#[must_use]
pub fn time_increments(game: &Game) -> Vec<u64> {
    game.players
        .iter()
        .filter(|player| player.is_human())
        .map(|player| game.time_increments.get(player.index).copied().unwrap_or(0))
        .collect()
}

pub(crate) fn deserialize_counters<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let counters = Vec::<u64>::deserialize(deserializer)?;
    if counters.iter().any(|&value| value > MAX_INCREMENT) {
        return Err(serde::de::Error::custom(
            "time increment is not a safe integer",
        ));
    }
    Ok(counters)
}

pub(crate) fn execute(mut game: Game, action: Action, actor: usize) -> Result<Game, String> {
    let eligible = game.context == GameContext::Play
        && !matches!(
            action,
            Action::Undo
                | Action::Redo
                | Action::StartTurn
                | Action::QueueChoice { .. }
                | Action::QueueMoves { .. }
        );
    let before_player = game.active_player();
    // Committing a simultaneous draft seat relinquishes control of that seat.
    // Revisions must not credit again, including when loading a pre-hook save.
    let draft_commit = game
        .civilization_draft
        .as_ref()
        .map(|draft| draft.ready.get(actor).is_some_and(|ready| !ready));
    game.timing_revealed = false;
    let mut next = try_execute_action(game, action, actor)?;
    let revealed = std::mem::take(&mut next.timing_revealed);
    let boundary = draft_commit.unwrap_or_else(|| {
        revealed || next.active_player() != before_player || next.state == GameState::Finished
    });
    if eligible
        && boundary
        && next
            .players
            .get(actor)
            .is_some_and(|player| player.is_human())
    {
        // Human seats precede NPCs and never disappear, including after a drop.
        let humans = next
            .players
            .iter()
            .filter(|player| player.is_human())
            .count();
        next.time_increments.resize(humans, 0);
        if let Some(counter) = next.time_increments.get_mut(actor) {
            *counter = counter.saturating_add(1).min(MAX_INCREMENT);
        }
    }
    Ok(next)
}
