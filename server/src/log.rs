use crate::advance::Advance;
use crate::card::{HandCard, HandCardLocation};
use crate::city::MoodState;
use crate::combat_stats::CombatStats;
use crate::events::EventOrigin;
use crate::map::Terrain;
use crate::movement::{MoveUnits, move_event_origin};
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::StatusPhaseStateType;
use crate::structure::Structure;
use crate::unit::Units;
use crate::{action::Action, game::Game};
use json_patch::PatchOperation;
use num::Zero;
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
    Setup(usize),
    StatusPhase(StatusPhaseStateType),
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ActionLogAction {
    pub action: Action,
    pub player: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<EventOrigin>,
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
    #[serde(default)]
    #[serde(skip_serializing_if = "usize::is_zero")]
    pub active_events: usize,
}

impl ActionLogAction {
    #[must_use]
    pub fn new(
        action: Action,
        player: usize,
        origin: Option<EventOrigin>,
        active_events: usize,
    ) -> Self {
        Self {
            action,
            player,
            origin,
            undo: Vec::new(),
            combat_stats: None,
            items: Vec::new(),
            log: Vec::new(),
            active_events,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ActionLogIncidentToken {
    Take(u8),
    NoChange,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ActionLogBalance {
    Gain,
    Loss,
    Pay, // Like loss, but for payment
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ActionLogEntryStructure {
    pub structure: Structure,
    pub balance: ActionLogBalance,
    pub position: Position,
    pub port_position: Option<Position>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ActionLogEntryMove {
    pub units: Units,
    pub start: Position,
    pub destination: Position,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embark_carrier_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ActionLogEntry {
    Action {
        balance: ActionLogBalance,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        amount: Option<u32>,
    },
    Resources {
        resources: ResourcePile,
        balance: ActionLogBalance,
    },
    Advance {
        advance: Advance,
        incident_token: ActionLogIncidentToken,
        balance: ActionLogBalance,
    },
    Units {
        units: Units,
        balance: ActionLogBalance,
        position: Position,
    },
    Structure(ActionLogEntryStructure),
    HandCard {
        card: HandCard,
        from: HandCardLocation,
        to: HandCardLocation,
    },
    MoodChange {
        city: Position,
        mood: MoodState,
    },
    Move(ActionLogEntryMove),
    Explore {
        tiles: Vec<(Position, Terrain)>,
    },
}

impl ActionLogEntry {
    #[must_use]
    pub fn balance(&self) -> Option<&ActionLogBalance> {
        match self {
            ActionLogEntry::Action { balance, .. }
            | ActionLogEntry::Resources { balance, .. }
            | ActionLogEntry::Advance { balance, .. }
            | ActionLogEntry::Units { balance, .. } => Some(balance),
            ActionLogEntry::Structure(s) => Some(&s.balance),
            ActionLogEntry::HandCard { .. }
            | ActionLogEntry::MoodChange { .. }
            | ActionLogEntry::Move { .. }
            | ActionLogEntry::Explore { .. } => None,
        }
    }

    #[must_use]
    pub fn resources(resources: ResourcePile, balance: ActionLogBalance) -> Self {
        Self::Resources { resources, balance }
    }

    #[must_use]
    pub fn units(units: Units, balance: ActionLogBalance, position: Position) -> Self {
        Self::Units {
            units,
            balance,
            position,
        }
    }

    #[must_use]
    pub fn advance(
        advance: Advance,
        balance: ActionLogBalance,
        incident_token: ActionLogIncidentToken,
    ) -> Self {
        Self::Advance {
            advance,
            incident_token,
            balance,
        }
    }

    #[must_use]
    pub fn structure(
        structure: Structure,
        balance: ActionLogBalance,
        position: Position,
        port_position: Option<Position>,
    ) -> Self {
        Self::Structure(ActionLogEntryStructure {
            structure,
            balance,
            position,
            port_position,
        })
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
    pub fn action(balance: ActionLogBalance, amount: u32) -> Self {
        Self::Action {
            balance,
            amount: if amount == 1 { None } else { Some(amount) },
        }
    }

    #[must_use]
    pub fn move_units(player: &Player, m: &MoveUnits) -> Self {
        Self::Move(ActionLogEntryMove {
            units: m
                .units
                .iter()
                .map(|unit| player.get_unit(*unit).unit_type)
                .collect::<Units>(),
            start: player.get_unit(m.units[0]).position,
            destination: m.destination,
            embark_carrier_id: m.embark_carrier_id,
        })
    }

    #[must_use]
    pub(crate) fn explore_tiles(tiles: Vec<(Position, Terrain)>) -> ActionLogEntry {
        Self::Explore { tiles }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ActionLogItem {
    pub player: usize,
    #[serde(flatten)]
    pub entry: ActionLogEntry,
    pub origin: EventOrigin,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<EventOrigin>,
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
    let active_events = game.events.len();
    let player = game.active_player();
    let origin = action_origin(&item, game.player(player)).clone();
    let i = game.log_index;
    let l = &mut current_turn_log_mut(game).actions;
    remove_redo_actions(l, i);
    l.push(ActionLogAction::new(item, player, origin, active_events));
    game.log_index += 1;
}

fn action_origin(a: &Action, player: &Player) -> Option<EventOrigin> {
    match a {
        Action::Playing(p) => Some(p.playing_action_type(player).origin(player).clone()),
        Action::Movement(_) => Some(move_event_origin()),
        Action::Undo => panic!("Unexpected undo in log"),
        Action::Redo => panic!("Unexpected redo in log"),
        Action::StartTurn | Action::Response(_) | Action::ChooseCivilization(_) => None,
    }
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

pub(crate) fn add_start_turn_action_if_needed(game: &mut Game, player: usize) {
    let p = current_turn_log_mut(game);
    if p.actions.is_empty() {
        p.actions
            .push(ActionLogAction::new(Action::StartTurn, player, None, 0));
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
    game.log_index = 0;
    game.undo_limit = 0;
    game.log
        .last_mut()
        .expect("action log should exist")
        .rounds
        .last_mut()
        .expect("round should exist")
        .turns
        .push(ActionLogTurn::new(turn_type));
}
