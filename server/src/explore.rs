use crate::content::advances::NAVIGATION;
use crate::game::{ExploreResolutionState, Game, GameState, MoveState, UndoContext};
use crate::map::{Block, BlockPosition, Map, Rotation, Terrain, UnexploredBlock};
use crate::player::Player;
use crate::position::Position;
use hex2d::{Angle, Back, Coordinate, Direction, Forward, Left, LeftBack, Right, RightBack, Spacing, Spin};
use itertools::Itertools;

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
    game.state = GameState::ExploreResolution(ExploreResolutionState {
        block: move_to.clone(),
        move_state: move_state.clone(),
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

    if is_any_ship(game, player_index, units)
        && game
            .map
            .tiles
            .get(&destination)
            .is_some_and(Terrain::is_land)
    {
        if ship_can_teleport {
            for (p, t) in block.block.tiles(&block.position, rotation) {
                if t.is_water() {
                    game.add_to_last_log_item(&format!(
                        ". Teleported ship from {destination} to {p}"
                    ));
                    game.move_units(player_index, units, p, None);
                    return;
                }
            }
            panic!("No water tile found to teleport ship");
        }
        game.add_to_last_log_item(". Ship can't move to the explored tile");
        return;
    }
    game.move_units(player_index, units, destination, None);
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
    water_has_neighbors(unexplored_block, rotation, |p| {
        map.tiles.get(p).is_some_and(Terrain::is_water)
    })
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
            if map.tiles.get(&n).is_some_and(Terrain::is_water) && !ocean.contains(&n) {
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

    game.add_to_last_log_item(&format!(". Explored tiles {s}"));
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

#[must_use]
pub fn can_reach(
    starting: Position,
    destination: Position,
    player: &Player,
    units: &[u32],
    map: &Map,
) -> bool {
    starting.is_neighbor(destination)
        || can_reach_with_navigation(player, units, map).contains(&destination)
}

#[must_use]
fn can_reach_with_navigation(player: &Player, units: &[u32], map: &Map) -> Vec<Position> {
    if !player.has_advance(NAVIGATION) {
        return vec![];
    }
    let ship = units.iter().find_map(|&id| {
        let unit = player.get_unit(id).expect("unit not found");
        if unit.unit_type.is_ship() {
            Some(unit.position)
        } else {
            None
        }
    });
    if let Some(ship) = ship {
        let mut destination = vec![];

        let ring = ship.coordinate().ring_iter(1, Spin::CCW(Direction::XY));
        for r in ring {
            // for outside in ship
            //     .neighbors()
            //     .into_iter()
            //     .filter(|n| !map.tiles.contains_key(n))
            // {
            if !map.tiles.contains_key(&Position::from_coordinate(r)) {
                continue;
            }

            let start = Position::from_coordinate(r);
            // for start in outside {
            if let Some(Terrain::Water) = map.tiles.get(&start) {
                destination.push(start);
            } else if start.is_neighbor(ship) {
                // let mut visited = ship
                //     .neighbors()
                //     .iter()
                //     .filter(|n| map.tiles.get(n).is_some_and(|t| t.is_water()))
                //     .copied()
                //     .collect::<Vec<_>>();
                // visited.retain(|n| n != &start);
                // visited.push(ship);

                let mut visited = vec![ship.coordinate()];
                let c = start.coordinate();
                let dir = ship.coordinate().direction_to_cw(c).unwrap();
                // let x = map.tiles.get(&Position::from_coordinate(c + (dir + Right))).is_none();
                // let y = map.tiles.get(&Position::from_coordinate(c + (dir + Left))).is_none();
                // assert!(x != y, "x={x} y={y}");

                walk(map, c, dir, &mut visited);
                let nav = |v: &&Coordinate| {
                    **v != ship.coordinate()
                        && map
                            .tiles
                            .get(&Position::from_coordinate(**v))
                            .is_some_and(|t| t.is_water() || t.is_unexplored())
                };
                let first = visited.iter().find(nav);
                let last = visited.iter().rfind(nav);
                // for v in &visited {
                //     println!("x={} y={}", v.x, v.y);
                // }

                if let Some(first) = first {
                    destination.push(Position::from_coordinate(*first));
                }
                if let Some(last) = last {
                    destination.push(Position::from_coordinate(*last));
                }

                break;
                // if let Some(d) = can_navigate_to_ocean(start, start, map, visited) {
                //     destination.push(d);
                // }
            };
            // }
        }
        return destination;
    }
    vec![]
}

#[must_use]
fn can_navigate_to_ocean(
    origin: Position,
    start: Position,
    map: &Map,
    mut visited: Vec<Position>,
) -> Option<Position> {
    // start is a land tile at the edge of the map
    if visited.contains(&start) {
        return None;
    }

    visited.push(start);

    let next: Vec<Position> = start
        .neighbors()
        .iter()
        .filter(|n| {
            !visited.contains(n)
                && map.tiles.contains_key(n)
                && n.neighbors().iter().any(|n| !map.tiles.contains_key(n))
        })
        .copied()
        .collect();

    for n in &next {
        if map
            .tiles
            .get(n)
            .is_some_and(|t| t.is_water() || t.is_unexplored())
        {
            return Some(*n);
        }
    }

    if next.is_empty() {
        return None;
    }
    can_navigate_to_ocean(origin, next[0], map, visited)
}

const ALL_ANGLES: [Angle; 5] = [RightBack, Right, Forward, Left, LeftBack];

const SIZE: f32 = 60.0;
const SPACING: Spacing = Spacing::FlatTop(SIZE);
fn center(c: Coordinate) -> Coordinate {
    let p = c.to_pixel(SPACING);
    Coordinate { x: p.0 as i32, y: p.1 as i32 }
}

fn walk(
    map: &Map,
    start: Coordinate,
    direction: Direction,
    // destination: Coordinate,
    visited: &mut Vec<Coordinate>,
)  {
    let CENTER = center(Position::from_offset("D4").coordinate());
    // if start == destination {
    //     return true;
    // }
    if visited.contains(&start) {
        return ;
    }
    visited.push(start);

    let option = &start
        .neighbors()
        .into_iter()
        .filter(|n| !visited.contains(n) && is_perim(map, n))
    .sorted_by_key(|n| -center(*n).distance(CENTER))
        .next();

    if let Some(n) = option {
        walk(map, *n, direction, visited);
    }

    // st.ring_iter(1, Spin::CCW(Direction::XY))
    //     for a in ALL_ANGLES.iter() {
    //         let new_dir = direction + *a;
    //         let new_pos = start + new_dir;
    //         if !visited.contains(&new_pos) && is_perim(map, &new_pos) {
    //             return walk(map, new_pos, new_dir, visited);
    //         }
    //     }

    // false

    // let right = direction + Angle::Right;
    // let f = start + direction;
    // let r = start + right;
    //
    // if map.tiles.contains_key(&Position::from_coordinate(r)) {
    //     walk(map, r, right,visited);
    // } else if map.tiles.contains_key(&Position::from_coordinate(f)) {
    //     walk(map, f, direction,  visited);
    // }

    // for d in Direction::all() {
    //     let n = start + *d;
    //     if map.tiles.get(&n).is_some_and(|t| t.is_land()) {
    //         if walk(map, n, destination, visited) {
    //             return true;
    //         }
    //     }
    // }

    // false
}

fn is_perim(map: &Map, coordinate: &Coordinate) -> bool {
    let p = Position::from_coordinate(*coordinate);
    let t = map.tiles.get(&p);
    t.is_some() && p.neighbors().iter().any(|n| map.tiles.get(n).is_none())
}

// fn FindHexCubePerimeterLoopOutside(
//     cubeCells: Vec<Coordinate>,
//     startCell: Coordinate,
//     map: &Map,
// ) -> Vec<Coordinate> {
//     let mut perim: Vec<Coordinate> = vec![];
//     let footCell = startCell;
//
//     let startHandCell = footCell + Direction::XY;
//     let handCell = startHandCell;
//     //
//     // if (cubeCells.Any(x => x == handCell))
//     // throw
//     // new
//     // Exception("Start Cell Must be the top right most cell");
//
//     let handMovedFromStartingLocation = false;
//     let finished = false;
//
//     //Yes, this happened to me. Still refining my actual regions and merging is apparently flawed.
//     if (cubeCells.len() == 1) {
//         // Debug.LogWarning("Only 1 Tile Perimeter");
//         return cubeCells;
//     }
//
//     perim.push(startCell);
//     while true {
//         let footMoved = false;
//
//         handCell
//             .directions_to(footCell)
//             .iter()
//             .for_each(|footDirection| {
//                 // Angle::Left.
//                 let newFootLocation = handCell. + footDirection;
//
//                 if map
//                     .tiles
//                     .contains_key(server::position::Position::from_coordinate(newFootLocation))
//                 {
//                     // if newFootLocation == footCell
//                     // return;
//                     //
//                     // //It's possible and common that we ended up crossing a single body of water
//                     // //The tile muse be connected
//                     // if footCell.distance(newFootLocation) > 1
//                     // return;
//                     //
//                     // footCell = newFootLocation;
//                     // perim.push(newFootLocation);
//                     // footMoved = true;
//                 }
//             });
//     }
// else if footMoved
//
//     if (cubeCells.Any(x => x == newFootLocation))
//     {
//         if (newFootLocation == footCell)
//         return;
//
//         //It's possible and common that we ended up crossing a single body of water
//         //The tile muse be connected
//         if (footCell.distance(newFootLocation) > 1)
//         return;
//
//         footCell = newFootLocation;
//         perim.push(newFootLocation);
//         footMoved = true;
//     } else if (footMoved)
//     return;
// });
//     //The starting direction is always relative to the hand
//     foreach ( let footDirection in CounterClockwiseDirections(handCell.CubeCoordDirection(footCell)))
//     {
//     let newFootLocation = handCell.GetNeighborCube(footDirection);
//     if (cubeCells.Any(x => x == newFootLocation))
//     {
//     if (newFootLocation == footCell)
//     continue;
//
//     //It's possible and common that we ended up crossing a single body of water
//     //The tile muse be connected
//     if (footCell.HexCubeDistance(newFootLocation) > 1)
//     continue;
//
//     footCell = newFootLocation;
//     perim.Add(newFootLocation);
//     footMoved = true;
//     }
//     else if (footMoved)
//     break;
//     }
//
//     let handMoved = false;
//
//     //The starting direction is always relative to the foot's.
//     foreach ( let handDirection in ClockwiseFromDirections(footCell.CubeCoordDirection(handCell)))
//     {
//     let newHandPosition = footCell.GetNeighborCube(handDirection);
//
//     //Just like the other distance check, we need to make sure that if the hand position is back to the original position
//     //that the current foot cell is a neighbor because it is possible that we are walking back out of an inlet.
//     if (newHandPosition == startHandCell & & footCell.HexCubeDistance(startCell) < = 1 & & handMovedFromStartingLocation)
//     {
//     finished = true;
//     break;
//     }
//
//     if (cubeCells.All(x => x != newHandPosition))
//     {
//     if (newHandPosition == handCell)
//     continue;
//
//     handMovedFromStartingLocation = true;
//     handCell = newHandPosition;
//     handMoved = true;
//     }
//     else if (handMoved)
//     {
//     break;
//     }
//     }
//
//     if ( ! handMoved)
//     throw new Exception();
//
// }
// while (!finished && perim.Count < MaxPerimeterResult);
//
//
// if (perim.Count >= MaxPerimeterResult)
// Debug.LogError("Cancelled out of the perimeter loop. Stuck.");
//
// let lastCell = perim.Last();
// if (lastCell == startCell)
// perim.RemoveAt(perim.Count - 1);
//
// return perim;
// perim
// }
