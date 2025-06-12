use crate::player::Player;

use crate::combat_stats::CombatStats;
use crate::events::EventOrigin;
use crate::playing_actions::PlayingActionType;
use crate::resource_pile::ResourcePile;
use crate::wonder::Wonder;
use crate::{action::Action, game::Game};
use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogAge {
    pub rounds: Vec<ActionLogRound>,
}

impl ActionLogAge {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { rounds: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogRound {
    pub players: Vec<ActionLogPlayer>,
}

impl ActionLogRound {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogPlayer {
    pub index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionLogAction>,
}

impl ActionLogPlayer {
    #[must_use]
    pub(crate) fn new(player: usize) -> Self {
        Self {
            actions: Vec::new(),
            index: player,
        }
    }

    pub(crate) fn action(&self, game: &Game) -> &ActionLogAction {
        &self.actions[game.action_log_index]
    }

    pub(crate) fn clear_undo(&mut self) {
        for item in &mut self.actions {
            item.undo.clear();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogAction {
    pub action: Action,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undo: Vec<PatchOperation>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combat_stats: Option<CombatStats>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wonder_built: Option<Wonder>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub completed_objectives: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ActionLogItem>,
}

impl ActionLogAction {
    #[must_use]
    pub fn new(action: Action) -> Self {
        Self {
            action,
            undo: Vec::new(),
            combat_stats: None,
            completed_objectives: Vec::new(),
            wonder_built: None,
            items: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionLogItem {
    GainResources {
        resources: ResourcePile,
        origin: EventOrigin,
    },
    LoseResources {
        resources: ResourcePile,
        origin: EventOrigin,
    },
}

#[derive(Serialize, Deserialize)]
pub struct LogSliceOptions {
    pub player: Option<usize>,
    pub start: usize,
    pub end: Option<usize>,
}

pub(crate) fn linear_action_log(game: &Game) -> Vec<Action> {
    game.action_log
        .iter()
        .flat_map(|age| {
            age.rounds.iter().flat_map(|round| {
                round
                    .players
                    .iter()
                    .flat_map(|player| player.actions.iter().map(|item| item.action.clone()))
            })
        })
        .collect()
}

pub(crate) fn modifier_suffix(
    player: &Player,
    action_type: &PlayingActionType,
    game: &Game,
) -> String {
    if let PlayingActionType::Custom(c) = action_type {
        let action_type1 = *c;
        format!(
            " using {}",
            player
                .custom_action_info(action_type1)
                .event_origin
                .name(game)
        )
    } else {
        String::new()
    }
}

pub(crate) fn add_log_action(game: &mut Game, item: Action) {
    let i = game.action_log_index;
    let l = &mut current_player_turn_log_mut(game).actions;
    if i < l.len() {
        // remove items from undo
        l.drain(i..);
    }
    l.push(ActionLogAction::new(item));
    game.action_log_index += 1;
}

pub(crate) fn add_action_log_item(game: &mut Game, item: ActionLogItem) {
    let p = current_player_turn_log_mut(game);
    if p.actions.is_empty() {
        p.actions.push(ActionLogAction::new(Action::StartTurn))
    }
    current_log_action_mut(game).items.push(item);
}

///
/// # Panics
/// Panics if the log entry does not exist
#[must_use]
pub fn current_player_turn_log(game: &Game) -> &ActionLogPlayer {
    game.action_log
        .last()
        .expect("state should exist")
        .rounds
        .last()
        .expect("state should exist")
        .players
        .last()
        .expect("state should exist")
}

///
/// # Panics
/// Panics if the log entry does not exist
pub fn current_player_turn_log_mut(game: &mut Game) -> &mut ActionLogPlayer {
    game.action_log
        .last_mut()
        .expect("age log should exist")
        .rounds
        .last_mut()
        .expect("round log should exist")
        .players
        .last_mut()
        .expect("player log should exist")
}

pub(crate) fn current_log_action_mut(game: &mut Game) -> &mut ActionLogAction {
    current_player_turn_log_mut(game)
        .actions
        .last_mut()
        .expect("actions empty")
}
