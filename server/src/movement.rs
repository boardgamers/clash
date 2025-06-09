use crate::map::Map;
use crate::map::Terrain::{Fertile, Forest, Mountain, Unexplored};
use crate::resource_pile::ResourcePile;
use crate::unit::{Unit, UnitType, Units, set_unit_position, ship_capacity};
use crate::utils;
use std::collections::HashSet;

use crate::combat::move_with_possible_combat;
use crate::consts::{ARMY_MOVEMENT_REQUIRED_ADVANCE, MOVEMENT_ACTIONS, STACK_LIMIT};
use crate::content::civilizations::vikings::is_ship_construction_move;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::explore::move_to_unexplored_tile;
use crate::game::GameState::Movement;
use crate::game::{Game, GameState};
use crate::move_routes::{MoveRoute, move_routes};
use crate::movement::MovementAction::{Move, Stop};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::MoveInfo;
use crate::position::Position;
use crate::unit::{carried_units, get_current_move};
use crate::wonder::Wonder;
use itertools::Itertools;
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

    let mut ask_conversion = vec![]; // from ship construction
    let mut to_ship = Units::empty();

    for unit_id in units {
        set_unit_position(player_index, *unit_id, to, game);
        let unit = game.players[player_index].get_unit_mut(*unit_id);
        if unit.is_ship() && game.map.is_land(to) {
            ask_conversion.push(*unit_id);
        } else if !unit.is_ship() && game.map.is_sea(to) && embark_carrier_id.is_none() {
            // from ship construction
            to_ship += &unit.unit_type;
            unit.unit_type = UnitType::Ship;
        }
        unit.carrier_id = embark_carrier_id;

        if let Some(terrain) = terrain_movement_restriction(&game.map, to, unit) {
            unit.movement_restrictions.push(terrain);
        }

        for id in carried_units(*unit_id, &game.players[player_index]) {
            set_unit_position(player_index, id, to, game);
        }
    }

    if !to_ship.is_empty() {
        game.add_to_last_log_item(&format!(" converting {} to ships", to_ship.to_string(None)));
        if let Movement(move_state) = &mut game.state {
            move_state.current_move = CurrentMove::Embark {
                source: from,
                destination: to,
            };
        } else {
            panic!("expected movement state, but got: {:?}", game.state)
        }
    }

    if !ask_conversion.is_empty() {
        on_ship_construction_conversion(game, player_index, ask_conversion);
    }
}

pub(crate) fn on_ship_construction_conversion(
    game: &mut Game,
    player_index: usize,
    ask_conversion: Vec<u32>,
) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |events| &mut events.ship_construction_conversion,
        ask_conversion,
        PersistentEventType::ShipConstructionConversion,
    );
}

#[derive(Clone, Debug)]
pub enum MoveDestination {
    Tile(Position, PaymentOptions),
    Carrier(u32),
}

#[derive(Clone, Debug)]
pub struct MoveDestinations {
    pub list: Vec<MoveDestination>,
    pub modifiers: HashSet<EventOrigin>,
}

impl MoveDestinations {
    #[must_use]
    pub fn new(list: Vec<MoveDestination>, modifiers: HashSet<EventOrigin>) -> Self {
        MoveDestinations { list, modifiers }
    }

    #[must_use]
    pub fn empty() -> Self {
        MoveDestinations::new(Vec::new(), HashSet::new())
    }
}

#[must_use]
pub fn possible_move_destinations(
    game: &Game,
    player_index: usize,
    units: &[u32],
    start: Position,
) -> MoveDestinations {
    let player = game.player(player_index);
    let mut modifiers = HashSet::new();

    let mut res = possible_move_routes(player, game, units, start, None)
        .unwrap_or_default()
        .into_iter()
        .map(|route| {
            modifiers.extend(route.cost.modifiers.clone());
            MoveDestination::Tile(route.destination, route.cost)
        })
        .collect::<Vec<_>>();

    player.units.iter().for_each(|u| {
        if u.is_ship()
            && possible_move_routes(player, game, units, start, Some(u.id))
                .is_ok_and(|v| v.iter().any(|route| route.destination == u.position))
        {
            res.push(MoveDestination::Carrier(u.id));
        }
    });
    MoveDestinations::new(res, modifiers)
}

/// # Errors
///
/// Will return `Err` if the unit cannot move.
///
/// # Panics
///
/// Panics if destination tile does not exist
pub fn possible_move_routes(
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

pub(crate) fn execute_movement_action(
    game: &mut Game,
    action: MovementAction,
    player_index: usize,
) -> Result<(), String> {
    let player = &game.player(player_index);
    game.add_info_log_item(
        &(match &action {
            Move(m) if m.units.is_empty() => {
                format!("{player} used a movement actions but moved no units")
            }
            Move(m) => move_action_log(game, player, m),
            Stop => format!("{player} ended the movement action"),
        }),
    );

    if let GameState::Playing = game.state {
        if game.actions_left == 0 {
            return Err("No actions left".to_string());
        }
        game.actions_left -= 1;
        game.state = GameState::Movement(MoveState::new());
    }

    match action {
        Move(m) => {
            execute_move_action(game, player_index, &m)?;

            if let Movement(state) = &game.state {
                let all_moves_used =
                    state.movement_actions_left == 0 && state.current_move == CurrentMove::None;
                if all_moves_used
                    || !has_movable_units(game, game.player(game.current_player_index))
                {
                    game.state = GameState::Playing;
                }
            }
        }
        Stop => {
            game.state = GameState::Playing;
        }
    }

    Ok(())
}

fn execute_move_action(game: &mut Game, player_index: usize, m: &MoveUnits) -> Result<(), String> {
    let player = &game.players[player_index];
    let starting_position =
        player
            .get_unit(*m.units.first().expect(
                "instead of providing no units to move a stop movement actions should be done",
            ))
            .position;
    let destinations = move_units_destinations(
        player,
        game,
        &m.units,
        starting_position,
        m.embark_carrier_id,
    )?;

    let (dest, result) = destinations
        .iter()
        .find(|(route, _)| route.destination == m.destination)
        .map_or_else(
            || {
                Err(format!(
                    "destination {} not found in {:?}",
                    m.destination, destinations
                ))
            },
            |(dest, r)| Ok((dest, r.clone())),
        )?;
    result?;

    let c = &dest.cost;
    if c.is_free() {
        assert_eq!(m.payment, ResourcePile::empty(), "payment should be empty");
    } else {
        game.players[player_index].pay_cost(c, &m.payment);
    }

    let current_move = get_current_move(
        game,
        &m.units,
        starting_position,
        m.destination,
        m.embark_carrier_id,
    );
    let Movement(move_state) = &mut game.state else {
        return Err("no move state".to_string());
    };
    move_state.moved_units.extend(m.units.iter());
    move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();

    if matches!(current_move, CurrentMove::None) || move_state.current_move != current_move {
        move_state.movement_actions_left -= 1;
        move_state.current_move = current_move;
    }
    if !starting_position.is_neighbor(m.destination) {
        // roads move ends the current move
        move_state.current_move = CurrentMove::None;
    }

    let dest_terrain = game
        .map
        .get(m.destination)
        .expect("destination should be a valid tile");

    if dest_terrain == &Unexplored {
        move_to_unexplored_tile(
            game,
            player_index,
            &m.units,
            starting_position,
            m.destination,
        );
    } else {
        move_with_possible_combat(game, player_index, m);
    }

    Ok(())
}

pub(crate) type MoveRoutes = Vec<(MoveRoute, Result<(), String>)>;

fn move_units_destinations(
    player: &Player,
    game: &Game,
    unit_ids: &[u32],
    start: Position,
    embark_carrier_id: Option<u32>,
) -> Result<MoveRoutes, String> {
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
        (carried_units(id, &game.players[player_index]).len() + units.len()) as u8
            > ship_capacity(player)
    }) {
        return Err("carrier capacity exceeded".to_string());
    }

    let carrier_position = embark_carrier_id.map(|id| player.get_unit(id).position);

    let mut stack_size = 0;
    let mut movement_restrictions = vec![];

    for unit in &units {
        movement_restrictions.extend(unit.movement_restrictions.iter());
        check_can_move(player, start, embark_carrier_id, unit)?;
        if unit.is_army_unit() {
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
            .filter(|unit| unit.is_army_unit() && !unit.is_transported())
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
        && movement_restrictions.contains(&(&MovementRestriction::Fertile))
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
        if !unit.is_land_based() {
            return Err("the unit should be land-based to embark".to_string());
        }
        let carrier = player.get_unit(embark_carrier_id);
        if !carrier.is_ship() {
            return Err("the carrier should be a ship".to_string());
        }
    }
    if unit.is_army_unit() && !player.can_use_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE) {
        return Err("army movement advance missing".to_string());
    }
    Ok(())
}

fn is_valid_movement_type(
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

    if !is_ship_construction_move(game, units, dest) {
        for unit in units {
            if unit.is_land_based() && game.map.is_sea(dest) {
                return Err("the destination should be land".to_string());
            }
            if unit.is_ship() && game.map.is_land(dest) {
                return Err("the destination should be sea".to_string());
            }
        }
    }
    Ok(())
}

fn terrain_movement_restriction(
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
        Forest if unit.is_army_unit() => Some(MovementRestriction::Forest),
        _ => None,
    }
}

fn has_movable_units(game: &Game, player: &Player) -> bool {
    player.units.iter().any(|unit| {
        let result = possible_move_routes(player, game, &[unit.id], unit.position, None);
        result.is_ok_and(|r| !r.is_empty()) || can_embark(game, player, unit)
    })
}

#[must_use]
fn can_embark(game: &Game, player: &Player, unit: &Unit) -> bool {
    unit.is_land_based()
        && player.units.iter().any(|u| {
            u.is_ship()
                && possible_move_routes(player, game, &[unit.id], u.position, Some(u.id)).is_ok()
        })
}

pub(crate) fn move_action_log(game: &Game, player: &Player, m: &MoveUnits) -> String {
    let units_str = m
        .units
        .iter()
        .map(|unit| player.get_unit(*unit).unit_type)
        .collect::<Units>()
        .to_string(Some(game));
    let start = player.get_unit(m.units[0]).position;
    let start_is_water = game.map.is_sea(start);
    let dest = m.destination;
    let t = game
        .map
        .get(dest)
        .expect("the destination position should be on the map");
    let (verb, suffix) = if start_is_water {
        if t.is_unexplored() || t.is_water() {
            ("sailed", "")
        } else {
            ("disembarked", "")
        }
    } else if m.embark_carrier_id.is_some() {
        ("embarked", "")
    } else if start.is_neighbor(dest) {
        ("marched", "")
    } else {
        ("marched", " on roads")
    };
    let payment = &m.payment;
    let cost = if payment.is_empty() {
        String::new()
    } else {
        format!(" for {payment}")
    };
    format!("{player} {verb} {units_str} from {start} to {dest}{suffix}{cost}",)
}
