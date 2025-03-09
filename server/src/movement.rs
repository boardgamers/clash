use crate::content::advances::{NAVIGATION, ROADS};
use crate::events::EventOrigin;
use crate::map::Map;
use crate::map::Terrain::{Forest, Mountain};
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use crate::unit::Unit;

use crate::consts::{ARMY_MOVEMENT_REQUIRED_ADVANCE, MOVEMENT_ACTIONS, SHIP_CAPACITY, STACK_LIMIT};
use crate::game::Game;
use crate::game::GameState::Movement;
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

pub(crate) fn take_move_state(game: &mut Game) -> MoveState {
    if let Movement(m) = game.pop_state() {
        m
    } else {
        panic!("no move state to pop");
    }
}

pub(crate) fn stop_current_move(game: &mut Game) {
    if let Movement(_) = game.state() {
        let mut move_state = take_move_state(game);
        move_state.current_move = CurrentMove::None;

        if move_state.movement_actions_left == 0 {
            return;
        }
        game.push_state(Movement(move_state));
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
    let from = p.get_unit(units[0]).position;
    let info = MoveInfo::new(player_index, units.to_vec(), from, to);
    game.trigger_event_with_game_value(player_index, |e| &mut e.before_move,  &info, &());

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
    let unit = game.players[player_index].get_unit_mut(unit_id);
    unit.position = destination;
    unit.carrier_id = embark_carrier_id;

    if let Some(terrain) = terrain_movement_restriction(&game.map, destination, unit) {
        unit.movement_restrictions.push(terrain);
    }

    for id in carried_units(unit_id, &game.players[player_index]) {
        game.players[player_index].get_unit_mut(id).position = destination;
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
    let (moved_units, movement_actions_left, current_move) = if let Movement(m) = &game.state() {
        (&m.moved_units, m.movement_actions_left, &m.current_move)
    } else {
        (&vec![], 1, &CurrentMove::None)
    };

    let units = unit_ids
        .iter()
        .map(|id| player.get_unit(*id))
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

    let carrier_position = embark_carrier_id.map(|id| player.get_unit(id).position);

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
            let carrier = player.get_unit(embark_carrier_id);
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

#[derive(Debug, Clone)]
pub struct MoveRoute {
    pub destination: Position,
    pub cost: PaymentOptions,
    pub stack_size_used: usize,
    pub ignore_terrain_movement_restrictions: bool,
    pub origin: Option<EventOrigin>,
}

impl MoveRoute {
    fn free(destination: Position, origin: Option<EventOrigin>) -> Self {
        Self {
            destination,
            cost: PaymentOptions::free(),
            stack_size_used: 0,
            ignore_terrain_movement_restrictions: false,
            origin,
        }
    }
}

pub(crate) fn is_valid_movement_type(
    game: &Game,
    units: &Vec<&Unit>,
    embark_position: Option<Position>,
    dest: Position,
) -> bool {
    if let Some(embark_position) = embark_position {
        return dest == embark_position;
    }
    units.iter().all(|unit| {
        if unit.unit_type.is_land_based() && game.map.is_sea(dest) {
            return false;
        }
        if unit.unit_type.is_ship() && game.map.is_land(dest) {
            return false;
        }
        true
    })
}

#[must_use]
pub(crate) fn move_routes(
    starting: Position,
    player: &Player,
    units: &[u32],
    game: &Game,
    embark_carrier_id: Option<u32>,
) -> Vec<MoveRoute> {
    let mut base: Vec<MoveRoute> = starting
        .neighbors()
        .iter()
        .filter(|&n| game.map.is_inside(*n))
        .map(|&n| MoveRoute::free(n, None))
        .collect();
    if player.has_advance(NAVIGATION) {
        base.extend(reachable_with_navigation(player, units, &game.map));
    }
    if player.has_advance(ROADS) && embark_carrier_id.is_none() {
        base.extend(reachable_with_roads(player, units, game));
    }
    base
}

#[must_use]
fn reachable_with_roads(player: &Player, units: &[u32], game: &Game) -> Vec<MoveRoute> {
    let start = units.iter().find_map(|&id| {
        let unit = player.get_unit(id);
        if unit.unit_type.is_land_based() {
            Some(unit.position)
        } else {
            None
        }
    });

    if let Some(start) = start {
        let map = &game.map;
        if map.is_sea(start) {
            // not for disembarking
            return vec![];
        };

        return start
            .neighbors()
            .into_iter()
            .flat_map(|middle| {
                // don't move over enemy units or cities
                let stack_size_used = player
                    .get_units(middle)
                    .iter()
                    .filter(|unit| unit.unit_type.is_army_unit())
                    .count();

                if map.is_land(middle) && game.enemy_player(player.index, middle).is_none() {
                    let mut dest: Vec<(Position, usize)> = middle
                        .neighbors()
                        .into_iter()
                        .map(move |n| (n, stack_size_used))
                        .collect();
                    dest.push((middle, stack_size_used));
                    dest.retain(|&(p, _)| p != start);
                    dest
                } else {
                    vec![]
                }
            })
            .into_group_map_by(|&(p, _)| p)
            .into_iter()
            .filter_map(|(destination, stack_sizes_used)| {
                // but can stop on enemy units
                if map.is_land(destination)
                    && (
                        // from or to owned city
                        player.try_get_city(start).is_some()
                            || player.try_get_city(destination).is_some()
                    )
                {
                    let stack_size_used =
                        stack_sizes_used.iter().map(|&(_, s)| s).min().expect("min");
                    let mut cost =
                        PaymentOptions::resources(ResourcePile::ore(1) + ResourcePile::food(1));
                    let origin = EventOrigin::Advance(ROADS.to_string());
                    cost.modifiers = vec![origin.clone()];
                    let route = MoveRoute {
                        destination,
                        cost,
                        stack_size_used,
                        ignore_terrain_movement_restrictions: true,
                        origin: Some(origin),
                    };
                    Some(route)
                } else {
                    None
                }
            })
            .collect();
    }
    vec![]
}

#[must_use]
fn reachable_with_navigation(player: &Player, units: &[u32], map: &Map) -> Vec<MoveRoute> {
    if !player.has_advance(NAVIGATION) {
        return vec![];
    }
    let ship = units.iter().find_map(|&id| {
        let unit = player.get_unit(id);
        if unit.unit_type.is_ship() {
            Some(unit.position)
        } else {
            None
        }
    });
    if let Some(ship) = ship {
        let start = ship.neighbors().into_iter().find(|n| map.is_outside(*n));
        if let Some(start) = start {
            let mut perimeter = vec![ship];

            add_perimeter(map, start, &mut perimeter);
            let can_navigate =
                |p: &Position| *p != ship && (map.is_sea(*p) || map.is_unexplored(*p));
            let first = perimeter.iter().copied().find(can_navigate);
            let last = perimeter.iter().copied().rfind(can_navigate);

            return vec![first, last]
                .into_iter()
                .flatten()
                .map(|destination| {
                    MoveRoute::free(
                        destination,
                        Some(EventOrigin::Advance(NAVIGATION.to_string())),
                    )
                })
                .collect();
        }
    }
    vec![]
}

fn add_perimeter(map: &Map, start: Position, perimeter: &mut Vec<Position>) {
    if perimeter.contains(&start) {
        return;
    }
    perimeter.push(start);

    let option = &start
        .neighbors()
        .into_iter()
        .filter(|n| {
            !perimeter.contains(n)
                && (map.is_inside(*n) && n.neighbors().iter().any(|n| map.is_outside(*n)))
        })
        // take with most outside neighbors first
        .sorted_by_key(|n| n.neighbors().iter().filter(|n| map.is_inside(**n)).count())
        .next();

    if let Some(n) = option {
        add_perimeter(map, *n, perimeter);
    }
}

pub(crate) fn terrain_movement_restriction(
    map: &Map,
    destination: Position,
    unit: &Unit,
) -> Option<MovementRestriction> {
    let terrain = map
        .get(destination)
        .expect("the destination position should exist on the map");
    match terrain {
        Mountain => Some(MovementRestriction::Mountain),
        Forest if unit.unit_type.is_army_unit() => Some(MovementRestriction::Forest),
        _ => None,
    }
}

pub(crate) fn has_movable_units(game: &Game, player: &Player) -> bool {
    player.units.iter().any(|unit| {
        move_units_destinations(player, game, &[unit.id], unit.position, None).is_ok()
            || can_embark(game, player, unit)
    })
}

fn can_embark(game: &Game, player: &Player, unit: &Unit) -> bool {
    unit.unit_type.is_land_based()
        && player.units.iter().any(|u| {
            u.unit_type.is_ship()
                && move_units_destinations(player, game, &[unit.id], u.position, Some(u.id)).is_ok()
        })
}
