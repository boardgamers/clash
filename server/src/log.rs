use crate::advance::Advance;
use crate::card::{HandCard, HandCardLocation};
use crate::city::MoodState;
use crate::combat_stats::CombatStats;
use crate::events::EventOrigin;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::structure::Structure;
use crate::unit::Units;
use crate::{action::Action, game::Game};
use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogAge {
    pub age: u32,
    pub rounds: Vec<ActionLogRound>,
}

impl ActionLogAge {
    #[must_use]
    pub(crate) fn new(age: u32) -> Self {
        Self {
            age,
            rounds: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogRound {
    pub round: u32,
    pub turns: Vec<ActionLogTurn>,
}

impl ActionLogRound {
    #[must_use]
    pub(crate) fn new(round: u32) -> Self {
        Self {
            round,
            turns: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum TurnType {
    Player(usize),
    Setup,
    StatusPhase,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogTurn {
    pub turn_type: TurnType,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionLogAction>,
}

impl ActionLogTurn {
    #[must_use]
    pub(crate) fn new(turn_type: TurnType) -> Self {
        Self {
            actions: Vec::new(),
            turn_type,
        }
    }

    pub(crate) fn last_action(&self, game: &Game) -> &ActionLogAction {
        &self.actions[game.log_index]
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ActionLogItem>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub log: Vec<String>,
}

impl ActionLogAction {
    #[must_use]
    pub fn new(action: Action) -> Self {
        Self {
            action,
            undo: Vec::new(),
            combat_stats: None,
            items: Vec::new(),
            log: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionLogBalance {
    Gain,
    Loss,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionLogEntry {
    Action {
        balance: ActionLogBalance,
    },
    Resources {
        resources: ResourcePile,
        balance: ActionLogBalance,
    },
    Advance {
        advance: Advance,
        take_incident_token: bool,
        balance: ActionLogBalance,
    },
    Units {
        units: Units,
        balance: ActionLogBalance,
    },
    Structure {
        structure: Structure,
        balance: ActionLogBalance,
        position: Position,
    },
    HandCard {
        card: HandCard,
        from: HandCardLocation,
        to: HandCardLocation,
    },
    MoodChange {
        city: Position,
        mood: MoodState,
    },
}

impl ActionLogEntry {
    #[must_use]
    pub fn resources(resources: ResourcePile, balance: ActionLogBalance) -> Self {
        Self::Resources { resources, balance }
    }

    #[must_use]
    pub fn units(units: Units, balance: ActionLogBalance) -> Self {
        Self::Units { units, balance }
    }

    #[must_use]
    pub fn advance(advance: Advance, balance: ActionLogBalance, take_incident_token: bool) -> Self {
        Self::Advance {
            advance,
            take_incident_token,
            balance,
        }
    }

    #[must_use]
    pub fn structure(structure: Structure, balance: ActionLogBalance, position: Position) -> Self {
        Self::Structure {
            structure,
            balance,
            position,
        }
    }

    #[must_use]
    pub fn hand_card(card: HandCard, from: HandCardLocation, to: HandCardLocation) -> Self {
        Self::HandCard { card, from, to }
    }

    #[must_use]
    pub fn mood_change(city: Position, mood: MoodState) -> Self {
        Self::MoodChange { city, mood }
    }

    #[must_use]
    pub fn action(balance: ActionLogBalance) -> Self {
        Self::Action { balance }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogItem {
    pub player: usize,
    #[serde(flatten)]
    pub entry: ActionLogEntry,
    origin: EventOrigin,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    modifiers: Vec<EventOrigin>,
}

impl ActionLogItem {
    #[must_use]
    pub fn new(
        player: usize,
        entry: ActionLogEntry,
        origin: EventOrigin,
        modifiers: Vec<EventOrigin>,
    ) -> Self {
        Self {
            player,
            entry,
            origin,
            modifiers,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LogSliceOptions {
    pub player: Option<usize>,
    pub start: usize,
    pub end: Option<usize>,
}

pub(crate) fn linear_action_log(game: &Game) -> Vec<Action> {
    game.log
        .iter()
        .flat_map(|age| {
            age.rounds.iter().flat_map(|round| {
                round
                    .turns
                    .iter()
                    .flat_map(|player| player.actions.iter().map(|item| item.action.clone()))
            })
        })
        .collect()
}

pub(crate) fn add_log_action(game: &mut Game, item: Action) {
    let i = game.log_index;
    let l = &mut current_turn_log_mut(game).actions;
    remove_redo_actions(l, i);
    l.push(ActionLogAction::new(item));
    game.log_index += 1;
}

fn remove_redo_actions(l: &mut Vec<ActionLogAction>, action_log_index: usize) {
    if action_log_index < l.len() {
        // remove items from undo
        for i in l.len()..action_log_index {
            let item = l.get(i).expect("should have action");
            if item.action != Action::StartTurn {
                l.pop();
            }
        }
    }
}

pub(crate) fn add_action_log_item(
    game: &mut Game,
    player: usize,
    entry: ActionLogEntry,
    origin: EventOrigin,
    modifiers: Vec<EventOrigin>,
) {
    current_action_log_mut(game)
        .items
        .push(ActionLogItem::new(player, entry, origin, modifiers));
}

pub(crate) fn add_start_turn_action_if_needed(game: &mut Game) {
    let p = current_turn_log_mut(game);
    if p.actions.is_empty() {
        p.actions.push(ActionLogAction::new(Action::StartTurn));
        game.log_index += 1;
    }
}

#[must_use]
pub(crate) fn current_turn_log_without_redo(game: &Game) -> ActionLogTurn {
    let mut log = current_turn_log(game).clone();
    remove_redo_actions(&mut log.actions, game.log_index);
    log
}

///
/// # Panics
/// Panics if the log entry does not exist
#[must_use]
pub(crate) fn current_turn_log(game: &Game) -> &ActionLogTurn {
    game.log
        .last()
        .expect("state should exist")
        .rounds
        .last()
        .expect("state should exist")
        .turns
        .last()
        .expect("state should exist")
}

///
/// # Panics
/// Panics if the log entry does not exist
pub fn current_turn_log_mut(game: &mut Game) -> &mut ActionLogTurn {
    game.log
        .last_mut()
        .expect("age log should exist")
        .rounds
        .last_mut()
        .expect("round log should exist")
        .turns
        .last_mut()
        .expect("player log should exist")
}

pub(crate) fn current_action_log_mut(game: &mut Game) -> &mut ActionLogAction {
    current_turn_log_mut(game)
        .actions
        .last_mut()
        .expect("actions empty")
}

pub(crate) fn add_round_log(game: &mut Game, round: u32) {
    game.log
        .last_mut()
        .expect("action log should exist")
        .rounds
        .push(ActionLogRound::new(round));
}

pub(crate) fn add_turn_log(game: &mut Game, turn_type: TurnType) {
    game.log
        .last_mut()
        .expect("action log should exist")
        .rounds
        .last_mut()
        .expect("round should exist")
        .turns
        .push(ActionLogTurn::new(turn_type));
}
