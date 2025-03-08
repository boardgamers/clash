use crate::action::{add_log_item_from_action, execute_movement_action, Action};
use crate::consts::MOVEMENT_ACTIONS;
use crate::content::custom_phase_actions::CurrentEventState;
use crate::game::Game;
use crate::game::GameState::Movement;
use crate::movement::{undo_move_units, MoveState};
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::MovementAction::Move;
use crate::unit::{MovementAction, UnitData};
use json_patch::{patch, PatchOperation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandUndoInfo {
    pub player: usize,
    pub info: HashMap<String, String>,
}

impl CommandUndoInfo {
    #[must_use]
    pub fn new(player: &Player) -> Self {
        Self {
            info: player.event_info.clone(),
            player: player.index,
        }
    }

    pub fn apply(&self, game: &mut Game, mut undo: CommandContext) {
        let player = game.get_player_mut(self.player);
        for (k, v) in undo.info.clone() {
            player.event_info.insert(k, v);
        }

        if undo.info != self.info || !undo.gained_resources.is_empty() {
            undo.info.clone_from(&self.info);
            game.push_undo_context(UndoContext::Command(undo));
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum UndoContext {
    FoundCity {
        settler: UnitData,
    },
    Recruit {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        #[serde(default)]
        replaced_units: Vec<UnitData>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        replaced_leader: Option<String>,
    },
    Movement {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        starting_position: Option<Position>,
        #[serde(flatten)]
        move_state: MoveState,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        disembarked_units: Vec<DisembarkUndoContext>,
    },
    WastedResources {
        resources: ResourcePile,
        player_index: usize,
    },
    IncreaseHappiness {
        angry_activations: Vec<Position>,
    },
    Event(Box<CurrentEventState>),
    Command(CommandContext),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DisembarkUndoContext {
    pub unit_id: u32,
    pub carrier_id: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandContext {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub info: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub gained_resources: ResourcePile,
}

impl CommandContext {
    #[must_use]
    pub fn new(info: HashMap<String, String>) -> Self {
        Self {
            info,
            gained_resources: ResourcePile::empty(),
        }
    }
}

const IGNORE_PATHS: [&str; 3] = ["/action_log", "/log/", "/actions_left"];

pub(crate) fn clean_patch(mut patch: Vec<PatchOperation>) -> Vec<PatchOperation> {
    patch.retain(|op| {
        IGNORE_PATHS
            .iter()
            .all(|ignore| !op.path().as_str().starts_with(ignore))
    });
    patch
}

pub(crate) fn undo(mut game: Game, player_index: usize) -> Game {
    game.action_log_index -= 1;
    game.log.remove(game.log.len() - 1);

    let item = game.action_log.last_mut().expect("should have action log");
    let p = std::mem::take(&mut item.undo);

    match &item.action {
        Action::Playing(action) => action.clone().undo(&mut game, player_index),
        Action::Undo => panic!("undo action can't be undone"),
        Action::Redo => panic!("redo action can't be undone"),
        _ => {} // Action::Movement(action) => {
                //     undo_movement_action(game, action.clone(), player_index);
                // }
                // Action::Response(action) => action.clone().undo(game, player_index),
    }

    let mut v = to_serde_value(&game);

    patch(&mut v, &p).unwrap_or_else(|e| panic!("could not patch game data: {e}"));

    // let string = v.to_string();
    // println!("after undo: {}", string);

    game = Game::from_data(serde_json::from_value(v).expect("should be able to deserialize game"));

    // let action = item.action.clone();
    //
    // let was_custom_phase = game.current_event_handler().is_some();
    // if was_custom_phase {
    //     game.current_events.pop();
    // }
    //

    //
    // maybe_undo_waste(game);
    //
    // while game.maybe_pop_undo_context(|_| false).is_some() {
    //     // pop all undo contexts until action start
    // }
    game
}

pub(crate) fn to_serde_value(game: &Game) -> Value {
    let s = serde_json::to_string(&game.cloned_data()).expect("game should be serializable");
    serde_json::from_str(&s).expect("game should be serializable")
}

fn maybe_undo_waste(game: &mut Game) {
    if let Some(UndoContext::WastedResources {
        resources,
        player_index,
    }) = game.maybe_pop_undo_context(|c| matches!(c, UndoContext::WastedResources { .. }))
    {
        game.players[player_index].gain_resources_in_undo(resources.clone());
    }
}

pub fn redo(game: &mut Game, player_index: usize) {
    let copy = game.action_log[game.action_log_index].clone();
    add_log_item_from_action(game, &copy.action);
    match &game.action_log[game.action_log_index].action.clone() {
        Action::Playing(action) => action.clone().execute(game, player_index),
        Action::Movement(action) => {
            execute_movement_action(game, action.clone(), player_index);
        }
        Action::Response(action) => action.clone().redo(game, player_index),
        Action::Undo => panic!("undo action can't be redone"),
        Action::Redo => panic!("redo action can't be redone"),
    }
    game.action_log_index += 1;
}

pub(crate) fn undo_commands(game: &mut Game, c: &CommandContext) {
    let p = game.active_player();
    game.players[p].event_info.clone_from(&c.info);
    game.players[p].lose_resources(c.gained_resources.clone());
}

fn undo_movement_action(game: &mut Game, action: MovementAction, player_index: usize) {
    let Some(UndoContext::Movement {
        starting_position,
        move_state,
        disembarked_units,
    }) = game.pop_undo_context()
    else {
        panic!("when undoing a movement action, the game should have stored movement context")
    };
    if let Move(m) = action {
        undo_move_units(
            game,
            player_index,
            m.units,
            starting_position
                .expect("undo context should contain the starting position if units where moved"),
        );
        game.players[player_index].gain_resources_in_undo(m.payment);
        for unit in disembarked_units {
            game.players[player_index]
                .get_unit_mut(unit.unit_id)
                .carrier_id = Some(unit.carrier_id);
        }
    }
    game.pop_state();
    if move_state.movement_actions_left == MOVEMENT_ACTIONS {
        game.actions_left += 1;
    } else {
        game.push_state(Movement(move_state));
    }
}
