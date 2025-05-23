use crate::events::EventOrigin;
use crate::map::Map;
use crate::map::Terrain::{Fertile, Forest, Mountain};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::resource_pile::ResourcePile;
use crate::unit::{Unit, set_unit_position};
use crate::utils;

use crate::advance::Advance;
use crate::consts::{ARMY_MOVEMENT_REQUIRED_ADVANCE, MOVEMENT_ACTIONS, SHIP_CAPACITY, STACK_LIMIT};
use crate::content::action_cards::negotiation::negotiations_partner;
use crate::content::incidents::great_diplomat::{DIPLOMAT_ID, diplomatic_relations_partner};
use crate::game::GameState::Movement;
use crate::game::{Game, GameState};
use crate::player::Player;
use crate::player_events::MoveInfo;
use crate::position::Position;
use crate::special_advance::SpecialAdvance;
use crate::unit::{carried_units, get_current_move};
use crate::wonder::Wonder;
use itertools::Itertools;
use pathfinding::prelude::astar;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct MoveUnits {
    pub units: Vec<u32>,
    pub destination: Position,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embark_carrier_id: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub payment: ResourcePile,
}

impl MoveUnits {
    #[must_use]
    pub fn new(
        units: Vec<u32>,
        destination: Position,
        embark_carrier_id: Option<u32>,
        payment: ResourcePile,
    ) -> Self {
        Self {
            units,
            destination,
            embark_carrier_id,
            payment,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum MovementAction {
    Move(MoveUnits),
    Stop,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum MovementRestriction {
    Battle,
    Mountain,
    Forest,
    Fertile,
}

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
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub great_warlord_used: bool,
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
            great_warlord_used: false,
        }
    }
}

pub(crate) fn stop_current_move(game: &mut Game) {
    if let Movement(move_state) = &mut game.state {
        move_state.current_move = CurrentMove::None;

        if move_state.movement_actions_left == 0 {
            game.state = GameState::Playing;
        }
    }
}

pub(crate) fn move_units(
    game: &mut Game,
    player_index: usize,
    units: &[u32],
    to: Position,
    embark_carrier_id: Option<u32>,
) {
    let p = game.player(player_index);
    let from = p.get_unit(units[0]).position;
    let info = MoveInfo::new(player_index, units.to_vec(), from, to);
    game.trigger_transient_event_with_game_value(player_index, |e| &mut e.before_move, &info, &());

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
    set_unit_position(player_index, unit_id, destination, game);
    let unit = game.players[player_index].get_unit_mut(unit_id);
    unit.carrier_id = embark_carrier_id;

    if let Some(terrain) = terrain_movement_restriction(&game.map, destination, unit) {
        unit.movement_restrictions.push(terrain);
    }

    for id in carried_units(unit_id, &game.players[player_index]) {
        set_unit_position(player_index, id, destination, game);
    }
}

/// # Errors
///
/// Will return `Err` if the unit cannot move.
///
/// # Panics
///
/// Panics if destination tile does not exist
pub fn possible_move_units_destinations(
    player: &Player,
    game: &Game,
    unit_ids: &[u32],
    start: Position,
    embark_carrier_id: Option<u32>,
) -> Result<Vec<MoveRoute>, String> {
    let destinations = move_units_destinations(player, game, unit_ids, start, embark_carrier_id)?;

    if destinations.is_empty() {
        return Err("no valid destinations".to_string());
    }
    Ok(destinations
        .into_iter()
        .filter_map(|(route, r)| r.is_ok().then_some(route))
        .collect_vec())
}

pub(crate) type MoveDestinations = Vec<(MoveRoute, Result<(), String>)>;

pub(crate) fn move_units_destinations(
    player: &Player,
    game: &Game,
    unit_ids: &[u32],
    start: Position,
    embark_carrier_id: Option<u32>,
) -> Result<MoveDestinations, String> {
    let (moved_units, movement_actions_left, current_move) = if let Movement(m) = &game.state {
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
        movement_restrictions.extend(unit.movement_restrictions.iter());
        check_can_move(player, start, embark_carrier_id, unit)?;
        if unit.unit_type.is_army_unit() {
            stack_size += 1;
        }
    }

    Ok(
        move_routes(start, player, unit_ids, game, embark_carrier_id, stack_size)
            .into_iter()
            .map(|route| {
                let result = move_route_result(
                    player,
                    game,
                    unit_ids,
                    start,
                    embark_carrier_id,
                    moved_units,
                    movement_actions_left,
                    current_move,
                    &units,
                    carrier_position,
                    stack_size,
                    &mut movement_restrictions,
                    &route,
                );
                (route, result)
            })
            .collect_vec(),
    )
}

#[allow(clippy::too_many_arguments)]
fn move_route_result(
    player: &Player,
    game: &Game,
    unit_ids: &[u32],
    start: Position,
    embark_carrier_id: Option<u32>,
    moved_units: &[u32],
    movement_actions_left: u32,
    current_move: &CurrentMove,
    units: &Vec<&Unit>,
    carrier_position: Option<Position>,
    stack_size: usize,
    movement_restrictions: &mut Vec<&MovementRestriction>,
    route: &MoveRoute,
) -> Result<(), String> {
    if !player.can_afford(&route.cost) {
        return Err("not enough resources".to_string());
    }

    is_move_restricted(player, game, stack_size, movement_restrictions, route)?;

    let dest = route.destination;
    if game.map.is_land(start)
        && player
            .get_units(dest)
            .iter()
            .filter(|unit| unit.unit_type.is_army_unit() && !unit.is_transported())
            .count()
            + stack_size
            > STACK_LIMIT
    {
        return Err("stack limit exceeded".to_string());
    }

    is_valid_movement_type(game, units, carrier_position, dest)?;

    if matches!(current_move, CurrentMove::None)
        || *current_move != get_current_move(game, unit_ids, start, dest, embark_carrier_id)
    {
        if movement_actions_left == 0 {
            return Err("no movement actions left".to_string());
        }

        if unit_ids.iter().any(|id| moved_units.contains(id)) {
            return Err("some units already moved".to_string());
        }
    }
    Ok(())
}

fn is_move_restricted(
    player: &Player,
    game: &Game,
    stack_size: usize,
    movement_restrictions: &Vec<&MovementRestriction>,
    route: &MoveRoute,
) -> Result<(), String> {
    if movement_restrictions.contains(&&MovementRestriction::Battle) {
        return Err("battle movement restriction".to_string());
    }
    let dest = route.destination;
    let attack = game.enemy_player(player.index, dest).is_some();
    if attack && game.map.is_land(dest) && stack_size == 0 {
        return Err("no army units to attack".to_string());
    }

    if !route.ignore_terrain_movement_restrictions {
        if movement_restrictions
            .iter()
            .contains(&&MovementRestriction::Mountain)
        {
            return Err("mountain movement restriction".to_string());
        }
        if attack
            && movement_restrictions
                .iter()
                .contains(&&MovementRestriction::Forest)
        {
            return Err("forest movement attack restriction".to_string());
        }
    }
    // this restriction can't be ignored
    if attack
        && game
            .try_get_any_city(dest)
            .is_some_and(|city| city.pieces.wonders.contains(&Wonder::GreatGardens))
        && movement_restrictions
            .iter()
            .any(|&r| r == &MovementRestriction::Fertile)
    {
        return Err("fertile movement attack great gardens restriction".to_string());
    }
    Ok(())
}

fn check_can_move(
    player: &Player,
    start: Position,
    embark_carrier_id: Option<u32>,
    unit: &Unit,
) -> Result<(), String> {
    if unit.position != start {
        return Err("the unit should be at the starting position".to_string());
    }
    if let Some(embark_carrier_id) = embark_carrier_id {
        if !unit.unit_type.is_land_based() {
            return Err("the unit should be land-based to embark".to_string());
        }
        let carrier = player.get_unit(embark_carrier_id);
        if !carrier.unit_type.is_ship() {
            return Err("the carrier should be a ship".to_string());
        }
    }
    if unit.unit_type.is_army_unit() && !player.can_use_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE) {
        return Err("army movement advance missing".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MoveRoute {
    pub destination: Position,
    pub cost: PaymentOptions,
    pub ignore_terrain_movement_restrictions: bool,
}

impl MoveRoute {
    fn free(destination: Position, origins: Vec<EventOrigin>) -> Self {
        let mut options = PaymentOptions::free();
        options.modifiers = origins;
        Self {
            destination,
            cost: options,
            ignore_terrain_movement_restrictions: false,
        }
    }
}

pub(crate) fn is_valid_movement_type(
    game: &Game,
    units: &Vec<&Unit>,
    embark_position: Option<Position>,
    dest: Position,
) -> Result<(), String> {
    if let Some(embark_position) = embark_position {
        return if dest == embark_position {
            Ok(())
        } else {
            Err("the destination should be the carrier position".to_string())
        };
    }
    for unit in units {
        if unit.unit_type.is_land_based() && game.map.is_sea(dest) {
            return Err("the destination should be land".to_string());
        }
        if unit.unit_type.is_ship() && game.map.is_land(dest) {
            return Err("the destination should be sea".to_string());
        }
    }
    Ok(())
}

#[must_use]
pub(crate) fn move_routes(
    starting: Position,
    player: &Player,
    units: &[u32],
    game: &Game,
    embark_carrier_id: Option<u32>,
    stack_size: usize,
) -> Vec<MoveRoute> {
    let mut base: Vec<MoveRoute> = starting
        .neighbors()
        .iter()
        .filter(|&n| game.map.is_inside(*n))
        .map(|&n| MoveRoute::free(n, vec![]))
        .collect();
    if player.can_use_advance(Advance::Navigation) {
        base.extend(reachable_with_navigation(player, units, &game.map));
    }
    if player.can_use_advance(Advance::Roads) && embark_carrier_id.is_none() {
        base.extend(reachable_with_roads(player, units, game, stack_size));
    }
    add_diplomatic_relations(player, game, &mut base);
    add_negotiations(player, game, &mut base);
    base
}

fn add_diplomatic_relations(player: &Player, game: &Game, base: &mut Vec<MoveRoute>) {
    if let Some(partner) = diplomatic_relations_partner(game, player.index) {
        let partner = game.player(partner);
        for r in base {
            if !partner.get_units(r.destination).is_empty() {
                r.cost.default += ResourcePile::culture_tokens(2);
                r.cost.modifiers.push(EventOrigin::Incident(DIPLOMAT_ID));
            }
        }
    }
}

fn add_negotiations(player: &Player, game: &Game, base: &mut Vec<MoveRoute>) {
    if let Some(partner) = negotiations_partner(game, player.index) {
        let partner = game.player(partner);
        base.retain(|r| partner.get_units(r.destination).is_empty());
    }
}

#[must_use]
fn reachable_with_roads(
    player: &Player,
    units: &[u32],
    game: &Game,
    stack_size: usize,
) -> Vec<MoveRoute> {
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
        }

        let roman_roads = player.has_special_advance(SpecialAdvance::RomanRoads);
        let mut routes: Vec<MoveRoute> = next_road_step(player, game, start, stack_size)
            .into_iter()
            .flat_map(|middle| next_road_step(player, game, middle, stack_size))
            .unique()
            .filter_map(|destination| {
                road_route(
                    player,
                    start,
                    destination,
                    roman_roads,
                    vec![EventOrigin::Advance(Advance::Roads)],
                )
            })
            .collect();

        if roman_roads {
            routes.extend(roman_roads_routes(player, game, start, stack_size));
        }

        return routes;
    }
    vec![]
}

const ROMAN_ROADS_LENGTH: u8 = 4;

fn roman_roads_routes(
    player: &Player,
    game: &Game,
    start: Position,
    stack_size: usize,
) -> Vec<MoveRoute> {
    if game.try_get_any_city(start).is_none() {
        return vec![];
    }

    player
        .cities
        .iter()
        .filter_map(|city| {
            let distance = city.position.distance(start) as u8;
            if distance > ROMAN_ROADS_LENGTH {
                return None;
            }
            let dst = city.position;

            let len = astar(
                &start,
                |p| {
                    next_road_step(player, game, *p, stack_size)
                        .iter()
                        .map(|&n| (n, 1))
                        .collect_vec()
                },
                |p| p.distance(dst),
                |&p| p == dst,
            )
            .map_or(u8::MAX, |(_path, len)| len as u8);
            if len > ROMAN_ROADS_LENGTH {
                return None;
            }
            road_route(
                player,
                start,
                dst,
                false,
                vec![
                    EventOrigin::Advance(Advance::Roads),
                    EventOrigin::SpecialAdvance(SpecialAdvance::RomanRoads),
                ],
            )
        })
        .collect()
}

fn road_route(
    player: &Player,
    start: Position,
    destination: Position,
    ignore_city_to_city: bool,
    modifiers: Vec<EventOrigin>,
) -> Option<MoveRoute> {
    if destination.distance(start) <= 1 {
        // can go directly without using roads
        return None;
    }

    // but can stop on enemy units

    let from_city = player.try_get_city(start).is_some();
    let to_city = player.try_get_city(destination).is_some();
    if !from_city && !to_city {
        return None;
    }
    if from_city && to_city && ignore_city_to_city {
        return None;
    }

    let mut cost = PaymentOptions::resources(
        player,
        PaymentReason::Move,
        ResourcePile::ore(1) + ResourcePile::food(1),
    );
    cost.modifiers = modifiers;
    Some(MoveRoute {
        destination,
        cost,
        ignore_terrain_movement_restrictions: true,
    })
}

fn next_road_step(
    player: &Player,
    game: &Game,
    from: Position,
    stack_size: usize,
) -> Vec<Position> {
    // don't move over enemy units or cities
    from.neighbors()
        .into_iter()
        .filter(|to| {
            let on_target = player
                .get_units(*to)
                .iter()
                .filter(|unit| unit.unit_type.is_army_unit())
                .count();
            game.map.is_land(*to)
                && game.enemy_player(player.index, *to).is_none()
                && on_target + stack_size <= STACK_LIMIT
        })
        .collect_vec()
}

#[must_use]
fn reachable_with_navigation(player: &Player, units: &[u32], map: &Map) -> Vec<MoveRoute> {
    if !player.can_use_advance(Advance::Navigation) {
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
                    MoveRoute::free(destination, vec![EventOrigin::Advance(Advance::Navigation)])
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
        Fertile => Some(MovementRestriction::Fertile),
        Mountain => Some(MovementRestriction::Mountain),
        Forest if unit.unit_type.is_army_unit() => Some(MovementRestriction::Forest),
        _ => None,
    }
}

pub(crate) fn has_movable_units(game: &Game, player: &Player) -> bool {
    player.units.iter().any(|unit| {
        let result =
            possible_move_units_destinations(player, game, &[unit.id], unit.position, None);
        result.is_ok_and(|r| !r.is_empty()) || can_embark(game, player, unit)
    })
}

#[must_use]
pub fn can_embark(game: &Game, player: &Player, unit: &Unit) -> bool {
    unit.unit_type.is_land_based()
        && player.units.iter().any(|u| {
            u.unit_type.is_ship()
                && possible_move_units_destinations(
                    player,
                    game,
                    &[unit.id],
                    u.position,
                    Some(u.id),
                )
                .is_ok()
        })
}
