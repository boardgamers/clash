use crate::action::{add_log_item_from_action, execute_movement_action, Action};
use crate::consts::MOVEMENT_ACTIONS;
use crate::content::custom_phase_actions::CustomPhaseEventState;
use crate::cultural_influence::{
    execute_cultural_influence_resolution_action, undo_cultural_influence_resolution_action,
};
use crate::explore::{explore_resolution, undo_explore_resolution, ExploreResolutionState};
use crate::game::Game;
use crate::game::GameState::{CulturalInfluenceResolution, ExploreResolution, Movement, Playing};
use crate::move_units::{undo_move_units, MoveState};
use crate::player::Player;
use crate::position::Position;
use crate::resource::check_for_waste;
use crate::resource_pile::ResourcePile;
use crate::unit::MovementAction::Move;
use crate::unit::{MovementAction, UnitData};
use serde::{Deserialize, Serialize};
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
    ExploreResolution(ExploreResolutionState),
    WastedResources {
        resources: ResourcePile,
        player_index: usize,
    },
    IncreaseHappiness {
        angry_activations: Vec<Position>,
    },
    InfluenceCultureResolution {
        roll_boost_cost: ResourcePile,
    },
    CustomPhaseEvent(Box<CustomPhaseEventState>),
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

pub(crate) fn undo(game: &mut Game, player_index: usize) {
    game.action_log_index -= 1;
    game.log.remove(game.log.len() - 1);
    let item = &game.action_log[game.action_log_index];
    game.undo_context_stack = item.undo.clone();
    let action = item.action.clone();

    let was_custom_phase = game.current_custom_phase_event().is_some();
    if was_custom_phase {
        game.custom_phase_state.pop();
    }

    match action {
        Action::Playing(action) => action.clone().undo(game, player_index, was_custom_phase),
        Action::StatusPhase(_) => panic!("status phase actions can't be undone"),
        Action::Movement(action) => {
            undo_movement_action(game, action.clone(), player_index);
        }
        Action::CulturalInfluenceResolution(action) => {
            undo_cultural_influence_resolution_action(game, action);
        }
        Action::ExploreResolution(_rotation) => {
            undo_explore_resolution(game, player_index);
        }
        Action::CustomPhaseEvent(action) => action.clone().undo(game, player_index),
        Action::Undo => panic!("undo action can't be undone"),
        Action::Redo => panic!("redo action can't be undone"),
    }

    maybe_undo_waste(game);

    while game.maybe_pop_undo_context(|_| false).is_some() {
        // pop all undo contexts until action start
    }
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
    match &game.action_log[game.action_log_index].action {
        Action::Playing(action) => action.clone().execute(game, player_index),
        Action::StatusPhase(_) => panic!("status phase actions can't be redone"),
        Action::Movement(action) => match &game.state {
            Playing => {
                execute_movement_action(game, action.clone(), player_index, MoveState::new());
            }
            Movement(m) => {
                execute_movement_action(game, action.clone(), player_index, m.clone());
            }
            _ => {
                panic!("movement actions can only be redone if the game is in a movement state")
            }
        },
        Action::CulturalInfluenceResolution(action) => {
            let CulturalInfluenceResolution(c) = &game.state else {
                panic!("cultural influence resolution actions can only be redone if the game is in a cultural influence resolution state");
            };
            execute_cultural_influence_resolution_action(
                game,
                *action,
                c.roll_boost_cost.clone(),
                c.target_player_index,
                c.target_city_position,
                c.city_piece,
                player_index,
            );
        }
        Action::ExploreResolution(rotation) => {
            let ExploreResolution(r) = &game.state else {
                panic!("explore resolution actions can only be redone if the game is in a explore resolution state");
            };
            explore_resolution(game, &r.clone(), *rotation);
        }
        Action::CustomPhaseEvent(action) => action.clone().redo(game, player_index),
        Action::Undo => panic!("undo action can't be redone"),
        Action::Redo => panic!("redo action can't be redone"),
    }
    game.action_log_index += 1;
    check_for_waste(game);
}

pub(crate) fn undo_commands(game: &mut Game, c: &CommandContext) {
    let p = game.current_player_index;
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
                .expect("unit should exist")
                .carrier_id = Some(unit.carrier_id);
        }
    }
    if move_state.movement_actions_left == MOVEMENT_ACTIONS {
        game.state = Playing;
        game.actions_left += 1;
    } else {
        game.state = Movement(move_state);
    }
}
