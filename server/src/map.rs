use hex2d::Angle;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::city::City;
use crate::city::MoodState::Happy;
use crate::player::Player;
use crate::position::Position;
use crate::unit::UnitType;

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

        let unexplored_blocks = setup
            .free_positions
            .iter()
            .map(|p| UnexploredBlock {
                position: p.clone(),
                block: BLOCKS[0].clone(), //todo random
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
                setup_home_city(&mut players[i], h.position.top_tile);
            });

        map
    }

    pub fn explore(&mut self, pos: &BlockPosition, block: &Block, rotation: Rotation) {
        self.unexplored_blocks
            .retain(|b| b.position.top_tile != pos.top_tile);

        self.add_block_tiles(pos, block, rotation);
    }

    fn add_block_tiles(&mut self, pos: &BlockPosition, block: &Block, rotation: Rotation) {
        let center = pos.top_tile;
        let left_relative = Position::new(-1, 1);
        let right_relative = Position::new(1, 0);
        let bottom = Position::new(0, 1);

        self.tiles.insert(center, block.terrain[0].clone());
        self.tiles.insert(
            rotate_around_center(center, left_relative, rotation),
            block.terrain[1].clone(),
        );
        self.tiles.insert(
            rotate_around_center(center, right_relative, rotation),
            block.terrain[2].clone(),
        );
        self.tiles.insert(
            rotate_around_center(center, bottom, rotation),
            block.terrain[3].clone(),
        );
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

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Terrain {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Exhausted(Box<Terrain>),
    Water,
    Unexplored,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    terrain: [Terrain; 4],
}

const UNEXPLORED_BLOCK: Block = Block {
    terrain: [
        Terrain::Unexplored,
        Terrain::Unexplored,
        Terrain::Unexplored,
        Terrain::Unexplored,
    ],
};

// by amount of water, descending
//     ┌──┐
//     │  │
// ┌───┐0 ┌───┐
// │1  │──│ 2 │
// └───┘  └───┘
//     │3 │
//     └──┘
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

pub type Rotation = usize;

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockPosition {
    pub top_tile: Position,
    pub rotation: Rotation,
}

#[derive(Serialize, Deserialize, Clone)]
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
                    top_tile: Position::from_offset("D8"),
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

fn rotate_around_center(center: Position, relative: Position, rotation: Rotation) -> Position {
    let pos = center.coordinate() + relative.coordinate();
    let coordinate = pos.rotate_around(center.coordinate(), Angle::all()[rotation]);
    Position::from_coordinate(coordinate)
}
