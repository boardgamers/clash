use crate::consts::{ARMY_MOVEMENT_REQUIRED_ADVANCE, MOVEMENT_ACTIONS, SHIP_CAPACITY, STACK_LIMIT};
use crate::game::GameState::Movement;
use crate::game::{Game, GameState};
use crate::movement::{
    has_movable_units, is_valid_movement_type, move_routes, terrain_movement_restriction, MoveRoute,
};
use crate::player::Player;
use crate::player_events::MoveInfo;
use crate::position::Position;
use crate::unit::{carried_units, get_current_move, MovementRestriction};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CurrentMove {
    #[default]
    None,
    Embark {
        source: Position,
        destination: Position,
    },
    Fleet {
        units: Vec<u32>,
    },
}

impl CurrentMove {
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, CurrentMove::None)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct MoveState {
    pub movement_actions_left: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub moved_units: Vec<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "CurrentMove::is_none")]
    pub current_move: CurrentMove,
}

impl Default for MoveState {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveState {
    #[must_use]
    pub fn new() -> Self {
        MoveState {
            movement_actions_left: MOVEMENT_ACTIONS,
            moved_units: Vec::new(),
            current_move: CurrentMove::None,
        }
    }
}

pub(crate) fn back_to_move(game: &mut Game, move_state: &MoveState, stop_current_move: bool) {
    let mut state = move_state.clone();
    if stop_current_move {
        state.current_move = CurrentMove::None;
    }
    // set state to Movement first, because that affects has_movable_units
    game.state = GameState::Movement(state);

    let all_moves_used =
        move_state.movement_actions_left == 0 && move_state.current_move == CurrentMove::None;
    if all_moves_used || !has_movable_units(game, game.get_player(game.current_player_index)) {
        game.state = GameState::Playing;
    }
}

pub(crate) fn move_units(
    game: &mut Game,
    player_index: usize,
    units: &[u32],
    to: Position,
    embark_carrier_id: Option<u32>,
) {
    let p = game.get_player(player_index);
    let from = p.get_unit(units[0]).expect("unit not found").position;
    let info = MoveInfo::new(player_index, units.to_vec(), from, to);
    game.trigger_command_event(player_index, |e| &mut e.before_move, &info);

    for unit_id in units {
        move_unit(game, player_index, *unit_id, to, embark_carrier_id);
    }
}

fn move_unit(
    game: &mut Game,
    player_index: usize,
    unit_id: u32,
    destination: Position,
    embark_carrier_id: Option<u32>,
) {
    let unit = game.players[player_index]
        .get_unit_mut(unit_id)
        .expect("the player should have all units to move");
    unit.position = destination;
    unit.carrier_id = embark_carrier_id;

    if let Some(terrain) = terrain_movement_restriction(&game.map, destination, unit) {
        unit.movement_restrictions.push(terrain);
    }

    for id in carried_units(unit_id, &game.players[player_index]) {
        game.players[player_index]
            .get_unit_mut(id)
            .expect("the player should have all units to move")
            .position = destination;
    }
}

pub(crate) fn undo_move_units(
    game: &mut Game,
    player_index: usize,
    units: Vec<u32>,
    starting_position: Position,
) {
    let Some(unit) = units.first() else {
        return;
    };
    let destination = game.players[player_index]
        .get_unit(*unit)
        .expect("there should be at least one moved unit")
        .position;

    for unit_id in units {
        let unit = game.players[player_index]
            .get_unit_mut(unit_id)
            .expect("the player should have all units to move");
        unit.position = starting_position;

        if let Some(terrain) = terrain_movement_restriction(&game.map, destination, unit) {
            unit.movement_restrictions
                .iter()
                .position(|r| r == &terrain)
                .map(|i| unit.movement_restrictions.remove(i));
        }

        if !game.map.is_sea(starting_position) {
            unit.carrier_id = None;
        }
        for id in &carried_units(unit_id, &game.players[player_index]) {
            game.players[player_index]
                .get_unit_mut(*id)
                .expect("the player should have all units to move")
                .position = starting_position;
        }
    }
}

/// # Errors
///
/// Will return `Err` if the unit cannot move.
///
/// # Panics
///
/// Panics if destination tile does not exist
pub fn move_units_destinations(
    player: &Player,
    game: &Game,
    unit_ids: &[u32],
    start: Position,
    embark_carrier_id: Option<u32>,
) -> Result<Vec<MoveRoute>, String> {
    let (moved_units, movement_actions_left, current_move) = if let Movement(m) = &game.state {
        (&m.moved_units, m.movement_actions_left, &m.current_move)
    } else {
        (&vec![], 1, &CurrentMove::None)
    };

    let units = unit_ids
        .iter()
        .map(|id| {
            player
                .get_unit(*id)
                .expect("the player should have all units to move")
        })
        .collect::<Vec<_>>();

    if units.is_empty() {
        return Err("no units to move".to_string());
    }
    if embark_carrier_id.is_some_and(|id| {
        let player_index = player.index;
        (carried_units(id, &game.players[player_index]).len() + units.len()) as u8 > SHIP_CAPACITY
    }) {
        return Err("carrier capacity exceeded".to_string());
    }

    let carrier_position = embark_carrier_id.map(|id| {
        player
            .get_unit(id)
            .expect("the player should have the carrier unit")
            .position
    });

    let mut stack_size = 0;
    let mut movement_restrictions = vec![];

    for unit in &units {
        if unit.position != start {
            return Err("the unit should be at the starting position".to_string());
        }
        movement_restrictions.extend(unit.movement_restrictions.iter());
        if let Some(embark_carrier_id) = embark_carrier_id {
            if !unit.unit_type.is_land_based() {
                return Err("the unit should be land based to embark".to_string());
            }
            let carrier = player
                .get_unit(embark_carrier_id)
                .ok_or("the player should have the carrier unit")?;
            if !carrier.unit_type.is_ship() {
                return Err("the carrier should be a ship".to_string());
            }
        }
        if unit.unit_type.is_army_unit() && !player.has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE) {
            return Err("army movement advance missing".to_string());
        }
        if unit.unit_type.is_army_unit() && !unit.unit_type.is_settler() {
            stack_size += 1;
        }
    }

    let destinations: Vec<MoveRoute> =
        move_routes(start, player, unit_ids, game, embark_carrier_id)
            .iter()
            .filter(|route| {
                if !player.can_afford(&route.cost) {
                    return false;
                }
                if movement_restrictions.contains(&&MovementRestriction::Battle) {
                    return false;
                }
                let dest = route.destination;
                let attack = game.enemy_player(player.index, dest).is_some();
                if attack && game.map.is_land(dest) && stack_size == 0 {
                    return false;
                }

                if !route.ignore_terrain_movement_restrictions {
                    if movement_restrictions
                        .iter()
                        .contains(&&MovementRestriction::Mountain)
                    {
                        return false;
                    }
                    if attack
                        && movement_restrictions
                            .iter()
                            .contains(&&MovementRestriction::Forest)
                    {
                        return false;
                    }
                }

                if game.map.is_land(start)
                    && player
                        .get_units(dest)
                        .iter()
                        .filter(|unit| unit.unit_type.is_army_unit() && !unit.is_transported())
                        .count()
                        + stack_size
                        + route.stack_size_used
                        > STACK_LIMIT
                {
                    return false;
                }

                if !is_valid_movement_type(game, &units, carrier_position, dest) {
                    return false;
                }

                if !matches!(current_move, CurrentMove::None)
                    && *current_move
                        == get_current_move(game, unit_ids, start, dest, embark_carrier_id)
                {
                    return true;
                }

                if movement_actions_left == 0 {
                    return false;
                }

                if unit_ids.iter().any(|id| moved_units.contains(id)) {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

    if destinations.is_empty() {
        return Err("no valid destinations".to_string());
    }
    Ok(destinations)
}
