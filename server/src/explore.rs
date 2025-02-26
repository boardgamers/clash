use crate::content::advances::NAVIGATION;
use crate::game::{Game, GameState};
use crate::map::{Block, BlockPosition, Map, Rotation, Terrain, UnexploredBlock};
use crate::move_units::{back_to_move, move_units, undo_move_units, CurrentMove, MoveState};
use crate::position::Position;
use crate::undo::UndoContext;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ExploreResolutionState {
    #[serde(flatten)]
    pub move_state: MoveState,
    pub block: UnexploredBlock,
    pub units: Vec<u32>,
    pub start: Position,
    pub destination: Position,
    pub ship_can_teleport: bool,
}

pub(crate) fn move_to_unexplored_tile(
    game: &mut Game,
    player_index: usize,
    units: &[u32],
    start: Position,
    destination: Position,
    move_state: &MoveState,
) -> bool {
    for b in &game.map.unexplored_blocks.clone() {
        for (position, _tile) in b.block.tiles(&b.position, b.position.rotation) {
            if position == destination {
                return move_to_unexplored_block(
                    game,
                    player_index,
                    b,
                    units,
                    start,
                    destination,
                    move_state,
                );
            }
        }
    }
    panic!("No unexplored tile at {destination}")
}

pub(crate) fn move_to_unexplored_block(
    game: &mut Game,
    player_index: usize,
    move_to: &UnexploredBlock,
    units: &[u32],
    start: Position,
    destination: Position,
    move_state: &MoveState,
) -> bool {
    let base = move_to.position.rotation;
    let opposite = (base + 3) as Rotation;

    let block = &move_to.block;
    let tiles = block.tiles(&move_to.position, base);
    let i = tiles
        .into_iter()
        .position(|(p, _)| p == destination)
        .expect("Destination not in block");
    let unrotated = &block.terrain[i];
    let rotated = &block.opposite(i);

    let ship_explore = is_any_ship(game, player_index, units);

    let instant_explore = |game: &mut Game, rotation: Rotation, ship_can_teleport| {
        game.lock_undo();
        move_to_explored_tile(
            game,
            move_to,
            rotation,
            player_index,
            units,
            destination,
            ship_can_teleport,
        );
        true // indicates to continue moving
    };

    let mut ship_can_teleport = false;

    if ship_explore {
        // first rule: find connected water
        let base_has_connected_sea = sea_is_connected(&game.map, start, move_to, base);
        let opposite_has_connected_sea = sea_is_connected(&game.map, start, move_to, opposite);
        if base_has_connected_sea != opposite_has_connected_sea {
            let rotation = if base_has_connected_sea {
                base
            } else {
                opposite
            };
            return instant_explore(game, rotation, true);
        }
        ship_can_teleport = base_has_connected_sea && opposite_has_connected_sea;
    } else {
        // first rule: don't move into water
        if unrotated.is_water() {
            return instant_explore(game, opposite, false);
        }
        if rotated.is_water() {
            return instant_explore(game, base, false);
        }
    }

    // second rule: water must be connected
    let base_has_water_neighbors = water_has_water_neighbors(&game.map, move_to, base);
    let opposite_has_water_neighbors = water_has_water_neighbors(&game.map, move_to, opposite);
    if base_has_water_neighbors != opposite_has_water_neighbors {
        let rotation = if base_has_water_neighbors {
            base
        } else {
            opposite
        };
        return instant_explore(game, rotation, false);
    }

    // third rule: prefer outside neighbors
    let base_has_outside_neighbors = water_has_outside_neighbors(&game.map, move_to, base);
    let opposite_has_outside_neighbors = water_has_outside_neighbors(&game.map, move_to, opposite);
    if base_has_outside_neighbors != opposite_has_outside_neighbors {
        let rotation = if base_has_outside_neighbors {
            base
        } else {
            opposite
        };
        return instant_explore(game, rotation, false);
    }

    game.lock_undo();
    let start = game
        .get_player(player_index)
        .get_unit(units[0])
        .expect("unit not found")
        .position;
    let mut state = move_state.clone();
    state.current_move = CurrentMove::None;
    game.state = GameState::ExploreResolution(ExploreResolutionState {
        block: move_to.clone(),
        move_state: state,
        units: units.to_vec(),
        start,
        destination,
        ship_can_teleport,
    });

    false // don't continue moving
}

fn move_to_explored_tile(
    game: &mut Game,
    block: &UnexploredBlock,
    rotation: Rotation,
    player_index: usize,
    units: &[u32],
    destination: Position,
    ship_can_teleport: bool,
) {
    add_block_tiles_with_log(game, &block.position, &block.block, rotation);

    if is_any_ship(game, player_index, units) && game.map.is_land(destination) {
        let player = game.get_player(player_index);
        let used_navigation = player.has_advance(NAVIGATION)
            && !player
                .get_unit(units[0])
                .expect("unit should exist")
                .position
                .is_neighbor(destination);

        if ship_can_teleport || used_navigation {
            for (p, t) in block.block.tiles(&block.position, rotation) {
                if t.is_water() {
                    game.add_info_log_item(&format!("Teleported ship from {destination} to {p}"));
                    move_units(game, player_index, units, p, None);
                    return;
                }
            }
        }
        game.add_info_log_item("Ship can't move to the explored tile");
        return;
    }
    move_units(game, player_index, units, destination, None);
}

pub fn is_any_ship(game: &Game, player_index: usize, units: &[u32]) -> bool {
    let p = game.get_player(player_index);
    units.iter().any(|&id| {
        p.get_unit(id)
            .expect("unit should exist")
            .unit_type
            .is_ship()
    })
}

#[must_use]
fn water_has_water_neighbors(
    map: &Map,
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
) -> bool {
    water_has_neighbors(unexplored_block, rotation, |p| map.is_sea(*p))
}

#[must_use]
fn sea_is_connected(
    map: &Map,
    start: Position,
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
) -> bool {
    let block = &unexplored_block.block;
    let tiles = block.tiles(&unexplored_block.position, rotation);
    let mut ocean = vec![start];
    grow_ocean(map, &mut ocean);
    tiles
        .into_iter()
        .any(|(p, t)| t.is_water() && p.neighbors().iter().any(|n| ocean.contains(n)))
}

fn grow_ocean(map: &Map, ocean: &mut Vec<Position>) {
    let mut i = 0;
    while i < ocean.len() {
        let pos = ocean[i];
        for n in pos.neighbors() {
            if map.is_sea(n) && !ocean.contains(&n) {
                ocean.push(n);
            }
        }
        i += 1;
    }
}

#[must_use]
fn water_has_outside_neighbors(
    map: &Map,
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
) -> bool {
    water_has_neighbors(unexplored_block, rotation, |p| !map.tiles.contains_key(p))
}

#[must_use]
fn water_has_neighbors(
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
    pred: impl Fn(&Position) -> bool,
) -> bool {
    let block = &unexplored_block.block;
    let tiles = block.tiles(&unexplored_block.position, rotation);
    tiles
        .into_iter()
        .any(|(p, t)| t.is_water() && p.neighbors().iter().any(&pred))
}

fn add_block_tiles_with_log(
    game: &mut Game,
    pos: &BlockPosition,
    block: &Block,
    rotation: Rotation,
) {
    game.map
        .unexplored_blocks
        .retain(|b| b.position.top_tile != pos.top_tile);

    let s = block
        .tiles(pos, rotation)
        .into_iter()
        .map(|(position, tile)| format!("{position}={tile:?}"))
        .sorted()
        .join(", ");

    game.add_info_log_item(&format!("Explored tiles {s}"));
    game.map.add_block_tiles(pos, block, rotation);
}

pub(crate) fn explore_resolution(game: &mut Game, r: &ExploreResolutionState, rotation: Rotation) {
    let unexplored_block = &r.block;
    let rotate_by = rotation - unexplored_block.position.rotation;
    let valid_rotation = rotate_by == 0 || rotate_by == 3;
    assert!(valid_rotation, "Invalid rotation {rotate_by}");

    move_to_explored_tile(
        game,
        unexplored_block,
        rotation,
        game.current_player_index,
        &r.units,
        r.destination,
        r.ship_can_teleport,
    );
    back_to_move(game, &r.move_state, true);
    game.push_undo_context(UndoContext::ExploreResolution(r.clone()));
}

pub(crate) fn undo_explore_resolution(game: &mut Game, player_index: usize) {
    let Some(UndoContext::ExploreResolution(s)) = game.pop_undo_context() else {
        panic!("when undoing explore resolution, the undo context stack should have an explore resolution")
    };

    let unexplored_block = &s.block;

    let block = &unexplored_block.block;
    block
        .tiles(
            &unexplored_block.position,
            unexplored_block.position.rotation,
        )
        .into_iter()
        .for_each(|(position, _tile)| {
            game.map.tiles.insert(position, Terrain::Unexplored);
        });

    game.map
        .add_unexplored_blocks(vec![unexplored_block.clone()]);

    undo_move_units(game, player_index, s.units.clone(), s.start);
    game.state = GameState::ExploreResolution(s);
}
