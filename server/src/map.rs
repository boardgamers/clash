use std::collections::HashMap;

use crate::city::City;
use crate::player::Player;
use crate::position::Position;
use crate::unit::UnitType;

#[derive(Clone)]
pub struct Map {
    pub tiles: HashMap<Position, Terrain>,
}

impl Map {
    #[must_use]
    pub fn new(tiles: HashMap<Position, Terrain>) -> Self {
        Self { tiles }
    }

    #[must_use]
    pub fn data(self) -> MapData {
        MapData {
            tiles: self.tiles.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> MapData {
        MapData {
            tiles: self.tiles.clone().into_iter().collect(),
        }
    }

    #[must_use]
    pub fn from_data(data: MapData) -> Self {
        Self {
            tiles: data.tiles.into_iter().collect(),
        }
    }
}

pub struct MapData {
    pub tiles: Vec<(Position, Terrain)>,
}

#[derive(Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Terrain {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Exhausted(Box<Terrain>),
    Water,
}

const RANDOM_TERRAIN: [Terrain; 9] = [
    Terrain::Barren,
    Terrain::Mountain,
    Terrain::Mountain,
    Terrain::Fertile,
    Terrain::Fertile,
    Terrain::Fertile,
    Terrain::Forest,
    Terrain::Forest,
    Terrain::Water,
];

const MAP_4_PLAYER: [&str; 72] = [
    "A4", "A6", "B1", "B3", "B4", "B5", "B6", "B8", "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8",
    "C9", "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "E2", "E3", "E4", "E5", "E6", "E7", "E8",
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "G2", "G3", "G4", "G5", "G6", "G7", "G8", "H1",
    "H2", "H3", "H4", "H5", "H6", "H7", "H8", "I1", "I2", "I3", "I4", "I5", "I6", "I7", "I8", "I9",
    "J1", "J3", "J4", "J5", "J6", "J8", "K4", "K6",
];

const FIXED_TERRAIN_2_PLAYER: [(&str, Terrain); 8] = [
    ("F1", Terrain::Fertile),
    ("F8", Terrain::Fertile),
    ("F2", Terrain::Barren),
    ("F7", Terrain::Barren),
    ("E2", Terrain::Forest),
    ("G8", Terrain::Forest),
    ("G2", Terrain::Mountain),
    ("E8", Terrain::Mountain),
];

pub fn maximum_size_2_player_random_map() -> HashMap<Position, Terrain> {
    let mut tiles = HashMap::new();

    tiles
}

pub fn setup_home_city(players: &mut [Player], player_index: usize, pos: &str) {
    let p = Position::from_offset(pos);
    players[player_index]
        .cities
        .push(City::new(player_index, p));
    players[player_index].add_unit(p, UnitType::Settler);
}
