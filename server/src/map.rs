use hex2d::Angle;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::city::City;
use crate::city::MoodState::Happy;
use crate::game::{ExploreResolutionState, Game, GameState, MoveState};
use crate::player::Player;
use crate::position::Position;
use crate::unit::UnitType;
use crate::utils::shuffle;

#[derive(Clone)]
pub struct Map {
    pub tiles: HashMap<Position, Terrain>,
    pub unexplored_blocks: Vec<UnexploredBlock>,
}

impl Map {
    #[must_use]
    pub fn new(tiles: HashMap<Position, Terrain>) -> Self {
        Self {
            tiles,
            unexplored_blocks: vec![],
        }
    }

    #[must_use]
    pub fn random_map(players: &mut [Player]) -> Self {
        let setup = get_map_setup(players.len());

        let blocks = shuffle(&mut BLOCKS.to_vec());
        let unexplored_blocks = setup
            .free_positions
            .iter()
            .enumerate()
            .map(|(i, p)| UnexploredBlock {
                position: p.clone(),
                block: blocks[i].clone(),
            })
            .collect_vec();

        let mut map = Self {
            tiles: HashMap::new(),
            unexplored_blocks: unexplored_blocks.clone(),
        };

        for b in unexplored_blocks {
            map.add_block_tiles(&b.position, &UNEXPLORED_BLOCK, b.position.rotation);
        }

        setup
            .home_positions
            .into_iter()
            .enumerate()
            .for_each(|(i, h)| {
                map.add_block_tiles(&h.position, &h.block, h.position.rotation);
                let position = h.block.tiles(&h.position, h.position.rotation)[0].0;
                setup_home_city(&mut players[i], position);
            });

        map
    }

    pub(crate) fn strip_secret(&mut self) {
        for b in &mut self.unexplored_blocks {
            b.block = UNEXPLORED_BLOCK.clone();
        }
    }

    pub(crate) fn explore_resolution(&mut self, r: &ExploreResolutionState, rotation: Rotation) {
        let position = &r.block.position;
        self.unexplored_blocks
            .retain(|b| b.position.top_tile != position.top_tile);
        let rotate_by = rotation - position.rotation;
        let valid_rotation = rotate_by == 0 || rotate_by == 3;
        assert!(valid_rotation, "Invalid rotation {rotate_by}");

        self.add_block_tiles(position, &r.block.block, rotation);
    }

    fn add_block_tiles(&mut self, pos: &BlockPosition, block: &Block, rotation: Rotation) {
        block
            .tiles(pos, rotation)
            .into_iter()
            .for_each(|(position, tile)| {
                self.tiles.insert(position, tile);
            });
    }

    #[must_use]
    pub fn data(self) -> MapData {
        MapData {
            tiles: self
                .tiles
                .into_iter()
                .sorted_by_key(|(position, _)| *position)
                .collect(),
            unexplored_blocks: self.unexplored_blocks,
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> MapData {
        MapData {
            tiles: self
                .tiles
                .clone()
                .into_iter()
                .sorted_by_key(|(position, _)| *position)
                .collect(),
            unexplored_blocks: self.unexplored_blocks.clone(),
        }
    }

    #[must_use]
    pub fn from_data(data: MapData) -> Self {
        Self {
            tiles: data.tiles.into_iter().collect(),
            unexplored_blocks: data.unexplored_blocks,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MapData {
    pub tiles: Vec<(Position, Terrain)>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unexplored_blocks: Vec<UnexploredBlock>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, PartialOrd, Ord, Debug)]
pub enum Terrain {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Exhausted(Box<Terrain>),
    Water,
    Unexplored,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Block {
    terrain: [Terrain; 4],
}

impl Block {
    #[must_use]
    pub fn tiles(&self, pos: &BlockPosition, rotation: Rotation) -> Vec<(Position, Terrain)> {
        let center = pos.top_tile;
        let flip = rotation > 2; // need to move the block to keep the tile in place
        BLOCK_RELATIVE_POSITIONS
            .into_iter()
            .enumerate()
            .map(|(i, relative)| {
                let tile = self.terrain[i].clone();
                let src = center.coordinate() + relative.coordinate();
                let mut dst =
                    src.rotate_around(center.coordinate(), Angle::from_int(rotation as i32));
                if flip {
                    dst = dst.neighbors()[rotation - 3];
                }
                (Position::from_coordinate(dst), tile)
            })
            .collect()
    }
}

//     ┌──┐
//     │  │
// ┌───┐0 ┌───┐
// │1  │──│ 2 │
// └───┘  └───┘
//     │3 │
//     └──┘
const BLOCK_RELATIVE_POSITIONS: [Position; 4] = [
    Position { q: 0, r: 0 },
    Position { q: -1, r: 1 },
    Position { q: 1, r: 0 },
    Position { q: 0, r: 1 },
];

const UNEXPLORED_BLOCK: Block = Block {
    terrain: [
        Terrain::Unexplored,
        Terrain::Unexplored,
        Terrain::Unexplored,
        Terrain::Unexplored,
    ],
};

// by amount of water, descending
const BLOCKS: [Block; 16] = [
    // 2 water tiles
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Water,
            Terrain::Forest,
            Terrain::Forest,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Mountain,
            Terrain::Water,
            Terrain::Mountain,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Water,
            Terrain::Mountain,
            Terrain::Fertile,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Forest,
            Terrain::Water,
            Terrain::Fertile,
        ],
    },
    // 1 water tile
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Fertile,
            Terrain::Fertile,
            Terrain::Barren,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Barren,
            Terrain::Forest,
            Terrain::Fertile,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Mountain,
            Terrain::Mountain,
            Terrain::Forest,
        ],
    },
    Block {
        terrain: [
            Terrain::Water,
            Terrain::Fertile,
            Terrain::Forest,
            Terrain::Mountain,
        ],
    },
    // water on left side
    Block {
        terrain: [
            Terrain::Forest,
            Terrain::Water,
            Terrain::Fertile,
            Terrain::Forest,
        ],
    },
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Water,
            Terrain::Fertile,
            Terrain::Barren,
        ],
    },
    Block {
        terrain: [
            Terrain::Mountain,
            Terrain::Water,
            Terrain::Fertile,
            Terrain::Mountain,
        ],
    },
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Water,
            Terrain::Barren,
            Terrain::Mountain,
        ],
    },
    //land only
    Block {
        terrain: [
            Terrain::Forest,
            Terrain::Mountain,
            Terrain::Fertile,
            Terrain::Mountain,
        ],
    },
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Fertile,
            Terrain::Fertile,
            Terrain::Forest,
        ],
    },
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Fertile,
        ],
    },
    Block {
        terrain: [
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Mountain,
            Terrain::Barren,
        ],
    },
];

// start city is at top
const STARTING_BLOCKS: [Block; 2] = [
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Forest,
            Terrain::Mountain,
            Terrain::Barren,
        ],
    },
    Block {
        terrain: [
            Terrain::Fertile,
            Terrain::Mountain,
            Terrain::Forest,
            Terrain::Barren,
        ],
    },
];

// 0 if top
// 1 if top right
// 2 if bottom right
// 3 if bottom
// 4 if bottom left
// 5 if top left
pub type Rotation = usize;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct BlockPosition {
    pub top_tile: Position,
    pub rotation: Rotation,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UnexploredBlock {
    pub position: BlockPosition,
    pub block: Block,
}

pub struct MapHomePosition {
    pub position: BlockPosition,
    pub block: Block,
}

pub struct MapSetup {
    pub home_positions: Vec<MapHomePosition>,
    pub free_positions: Vec<BlockPosition>,
}

#[must_use]
pub(crate) fn get_map_setup(player_count: usize) -> MapSetup {
    let setup = vec![MapSetup {
        home_positions: vec![
            MapHomePosition {
                position: BlockPosition {
                    top_tile: Position::from_offset("D1"),
                    rotation: 0,
                },
                block: STARTING_BLOCKS[0].clone(),
            },
            MapHomePosition {
                position: BlockPosition {
                    top_tile: Position::from_offset("D7"),
                    rotation: 3,
                },
                block: STARTING_BLOCKS[0].clone(),
            },
        ],
        free_positions: vec![
            BlockPosition {
                top_tile: Position::from_offset("B2"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("F2"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("D3"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("B4"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("F4"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("D5"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("B6"),
                rotation: 0,
            },
            BlockPosition {
                top_tile: Position::from_offset("F6"),
                rotation: 0,
            },
        ],
    }];
    setup
        .into_iter()
        .find(|s| s.home_positions.len() == player_count)
        .expect("No setup for this player count")
}

pub fn setup_home_city(player: &mut Player, pos: Position) {
    let mut city = City::new(player.index, pos);
    city.mood_state = Happy;
    player.cities.push(city);
    player.add_unit(pos, UnitType::Settler);
}

pub(crate) fn move_to_unexplored_tile(
    game: &mut Game,
    player_index: usize,
    move_to: Position,
    move_state: &MoveState,
) -> bool {
    for b in &game.map.unexplored_blocks.clone() {
        for (position, _tile) in b.block.tiles(&b.position, b.position.rotation) {
            if position == move_to {
                return move_to_unexplored_block(game, player_index, b, move_state);
            }
        }
    }
    panic!("No unexplored tile at {move_to}")
}

pub(crate) fn move_to_unexplored_block(
    game: &mut Game,
    _player_index: usize,
    move_to: &UnexploredBlock,
    move_state: &MoveState,
) -> bool {
    // todo: check if only one position is possible => return false

    game.lock_undo();
    game.state = GameState::ExploreResolution(ExploreResolutionState {
        block: move_to.clone(),
        move_state: move_state.clone(),
    });

    true
}
