use crate::game::{ExploreResolutionState, Game, GameState, MoveState, UndoContext};
use crate::map::{Block, BlockPosition, Map, Rotation, Terrain, UnexploredBlock};
use crate::position::Position;
use itertools::Itertools;

pub(crate) fn move_to_unexplored_tile(
    game: &mut Game,
    player_index: usize,
    units: &[u32],
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

    // first rule: don't move into water
    if unrotated.is_water() {
        return instant_explore(game, move_to, opposite);
    }
    if rotated.is_water() {
        return instant_explore(game, move_to, base);
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
        return instant_explore(game, move_to, rotation);
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
        return instant_explore(game, move_to, rotation);
    }

    game.lock_undo();
    let start = game
        .get_player(player_index)
        .get_unit(units[0])
        .expect("unit not found")
        .position;
    game.state = GameState::ExploreResolution(ExploreResolutionState {
        block: move_to.clone(),
        move_state: move_state.clone(),
        units: units.to_vec(),
        start,
        destination,
    });

    true
}

fn instant_explore(game: &mut Game, h: &UnexploredBlock, rotation: Rotation) -> bool {
    add_block_tiles_with_log(game, &h.position, &h.block, rotation);
    game.lock_undo();
    false // indicates to continue moving
}

fn water_has_water_neighbors(
    map: &Map,
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
) -> bool {
    has_neighbors(unexplored_block, rotation, |p| {
        map.tiles
            .get(p)
            .map_or(false, super::map::Terrain::is_water)
    })
}

fn water_has_outside_neighbors(
    map: &Map,
    unexplored_block: &UnexploredBlock,
    rotation: Rotation,
) -> bool {
    has_neighbors(unexplored_block, rotation, |p| !map.tiles.contains_key(p))
}

fn has_neighbors(
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

    game.add_to_last_log_item(&format!(". Explored tiles {s}"));
    game.map.add_block_tiles(pos, block, rotation);
}

pub(crate) fn explore_resolution(game: &mut Game, r: &ExploreResolutionState, rotation: Rotation) {
    let position = &r.block.position;
    let rotate_by = rotation - position.rotation;
    let valid_rotation = rotate_by == 0 || rotate_by == 3;
    assert!(valid_rotation, "Invalid rotation {rotate_by}");

    add_block_tiles_with_log(game, position, &r.block.block, rotation);
    game.move_units(game.current_player_index, &r.units, r.destination);
    game.back_to_move(&r.move_state);
    game.push_undo_context(UndoContext::ExploreResolution(r.clone()));
}

pub(crate) fn undo_explore_resolution(game: &mut Game, player_index: usize) {
    let Some(UndoContext::ExploreResolution(s)) = game.undo_context_stack.pop() else {
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

    game.undo_move_units(player_index, s.units.clone(), s.start);
    game.state = GameState::ExploreResolution(s);
}
