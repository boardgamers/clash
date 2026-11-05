//! Private live knowledge, deliberately outside undo patches and public history.
use crate::content::custom_actions::{CustomActionType, SpecialAction};
use crate::game::{Game, GameContext};
use crate::playing_actions::PlayingActionType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalendarKnowledge {
    pub player: usize,
    pub turn: usize,
    /// Last disclosed card, retained internally after a draw/burial for Undo.
    /// Filtering hides it while it is not the actual top; never learns a replacement.
    pub incident: Option<u8>,
}

pub(crate) fn turn(game: &Game) -> usize {
    game.log
        .iter()
        .flat_map(|a| &a.rounds)
        .map(|r| r.turns.len())
        .sum()
}

pub(crate) fn eligible(game: &Game, player: usize) -> bool {
    game.civilization_draft.is_none()
        && player == game.current_player_index
        && player == game.active_player()
        && game
            .player(player)
            .special_actions
            .contains_key(&SpecialAction::Custom(CustomActionType::Calendar))
        && PlayingActionType::Special(SpecialAction::Custom(CustomActionType::Calendar))
            .is_available(game, player)
            .is_ok()
}

pub(crate) fn known(game: &Game, player: usize) -> Option<u8> {
    let knowledge = game.calendar_knowledge.as_ref()?;
    if knowledge.player != player || knowledge.turn != turn(game) {
        return None;
    }
    let id = knowledge.incident?;
    // Filtered clients use only the explicit disclosure, never their shuffled pile.
    (game.secrets_stripped || game.incidents_left.first() == Some(&id)).then_some(id)
}

pub(crate) fn disclose(game: &mut Game) {
    if game.context != GameContext::Play || game.secrets_stripped {
        return;
    }
    let player = game.current_player_index;
    if !eligible(game, player) {
        return;
    }
    let turn = turn(game);
    if game
        .calendar_knowledge
        .as_ref()
        .is_some_and(|k| k.turn == turn && k.player == player)
    {
        return;
    }
    // Empty piles deliberately retain the existing explicit authoritative reveal.
    game.calendar_knowledge = Some(CalendarKnowledge {
        player,
        turn,
        incident: game.incidents_left.first().copied(),
    });
    if !game.incidents_left.is_empty() {
        game.lock_undo();
    }
}

pub(crate) fn remember(game: &mut Game, player: usize) {
    game.calendar_knowledge = Some(CalendarKnowledge {
        player,
        turn: turn(game),
        incident: game.incidents_left.first().copied(),
    });
}

pub(crate) fn strip(game: &mut Game, viewer: Option<usize>) {
    if viewer != Some(game.current_player_index)
        || game
            .calendar_knowledge
            .as_ref()
            .is_some_and(|k| Some(k.player) != viewer)
    {
        game.calendar_knowledge = None;
    } else if !game.secrets_stripped {
        let id = viewer.and_then(|p| known(game, p));
        if let Some(k) = &mut game.calendar_knowledge {
            k.incident = id;
        }
    }
    game.secrets_stripped = true;
}
