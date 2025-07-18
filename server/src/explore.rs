use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use crate::events::EventPlayer;
use crate::game::Game;
use crate::map::{Block, BlockPosition, Map, Rotation, UnexploredBlock};
use crate::movement::{move_units, stop_current_move};
use crate::position::Position;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ExploreResolutionState {
    pub block: UnexploredBlock,
    pub units: Vec<u32>,
    pub start: Position,
    pub destination: Option<Position>,
    pub ship_can_teleport: bool,
}

pub(crate) fn move_to_unexplored_tile(
    game: &mut Game,
    player: &EventPlayer,
    units: &[u32],
    start: Position,
    destination: Position,
) {
    stop_current_move(game);

    for b in &game.map.unexplored_blocks.clone() {
        for (position, _tile) in b.block.tiles(&b.position, b.position.rotation) {
            if position == destination {
                move_to_unexplored_block(game, player, b, units, start, Some(destination));
                return;
            }
        }
    }
    panic!("No unexplored tile at {destination}")
}

pub(crate) fn move_to_unexplored_block(
    game: &mut Game,
    player: &EventPlayer,
    move_to: &UnexploredBlock,
    units: &[u32],
    start: Position,
    destination: Option<Position>,
) {
    game.information_revealed(); // tile is revealed

    let base = move_to.position.rotation;
    let opposite = (base + 3).rem_euclid(6) as Rotation;

    let block = &move_to.block;
    let tiles = block.tiles(&move_to.position, base);

    let ship_explore = is_any_ship(game, player.index, units);

    let instant_explore = |game: &mut Game, rotation: Rotation, ship_can_teleport| {
        add_block_tiles_with_log(game, player, &move_to.position, &move_to.block, rotation);
        if let Some(destination) = destination {
            move_to_explored_tile(
                game,
                move_to,
                rotation,
                player,
                units,
                destination,
                ship_can_teleport,
            );
        }
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
    } else if let Some(destination) = destination {
        let i = tiles
            .into_iter()
            .position(|(p, _)| p == destination)
            .expect("Destination not in block");
        let t = &block.terrain[i];
        let rotated = &block.opposite(i);

        // first rule: don't move into water
        if t.is_water() {
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

    let resolution_state = ExploreResolutionState {
        block: move_to.clone(),
        units: units.to_vec(),
        start,
        destination,
        ship_can_teleport,
    };
    ask_explore_resolution(game, player.index, resolution_state);
}

pub(crate) fn ask_explore_resolution(
    game: &mut Game,
    player_index: usize,
    resolution_state: ExploreResolutionState,
) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |events| &mut events.explore_resolution,
        resolution_state,
        PersistentEventType::ExploreResolution,
    );
}

fn move_to_explored_tile(
    game: &mut Game,
    block: &UnexploredBlock,
    rotation: Rotation,
    player: &EventPlayer,
    units: &[u32],
    destination: Position,
    ship_can_teleport: bool,
) {
    if is_any_ship(game, player.index, units) && game.map.is_land(destination) {
        let p = player.get(game);
        let used_navigation = p.can_use_advance(Advance::Navigation)
            && !p.get_unit(units[0]).position.is_neighbor(destination);

        if ship_can_teleport || used_navigation {
            for (p, t) in block.block.tiles(&block.position, rotation) {
                if t.is_water() {
                    player.log(game, &format!("Teleported ship from {destination} to {p}"));
                    move_units(game, player.index, units, p, None);
                    return;
                }
            }
        }
        player.log(game, "Ship can't move to the explored tile");
        return;
    }
    move_units(game, player.index, units, destination, None);
}

pub fn is_any_ship(game: &Game, player_index: usize, units: &[u32]) -> bool {
    let p = game.player(player_index);
    units.iter().any(|&id| p.get_unit(id).is_ship())
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
    p: &EventPlayer,
    pos: &BlockPosition,
    block: &Block,
    rotation: Rotation,
) {
    game.map
        .unexplored_blocks
        .retain(|b| b.position.top_tile != pos.top_tile);

    let tiles = block.tiles(pos, rotation);

    let s = tiles
        .into_iter()
        .map(|(position, tile)| format!("{position}={tile}"))
        .sorted()
        .join(", ");

    p.log(game, &format!("Explored tiles {s}"));
    game.map.add_block_tiles(pos, block, rotation);
}

pub(crate) fn explore_resolution() -> Ability {
    Ability::builder(
        "Explore Resolution",
        "Select a rotation for the unexplored tiles",
    )
    .add_persistent_event_listener(
        |e| &mut e.explore_resolution,
        0,
        move |_game, _, _state| Some(PersistentEventRequest::ExploreResolution),
        move |game, p, action, _request, r| {
            let EventResponse::ExploreResolution(rotation) = action else {
                panic!("Invalid action");
            };

            p.log(game, &format!("Chose rotation {rotation}"));
            let unexplored_block = &r.block;
            let rotate_by =
                ((rotation as i8) - (unexplored_block.position.rotation as i8)).rem_euclid(6);
            let valid_rotation = rotate_by == 0 || rotate_by == 3;
            assert!(valid_rotation, "Invalid rotation {rotate_by}");

            add_block_tiles_with_log(
                game,
                p,
                &unexplored_block.position,
                &unexplored_block.block,
                rotation,
            );
            if let Some(destination) = r.destination {
                move_to_explored_tile(
                    game,
                    unexplored_block,
                    rotation,
                    p,
                    &r.units,
                    destination,
                    r.ship_can_teleport,
                );
            }
        },
    )
    .build()
}
