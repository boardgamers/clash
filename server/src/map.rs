use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::position::Position;

pub struct Map {
    pub tiles: HashMap<Position, Landmark>,
}

impl Map {
    pub fn new(tiles: HashMap<Position, Landmark>) -> Self {
        Self { tiles }
    }

    pub fn data(self) -> MapData {
        MapData {
            tiles: self.tiles.into_iter().collect(),
        }
    }

    pub fn from_data(data: MapData) -> Self {
        Self {
            tiles: data.tiles.into_iter().collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MapData {
    pub tiles: Vec<(Position, Landmark)>,
}

#[derive(Serialize, Deserialize, Hash)]
pub enum Landmark {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Unusable,
    Water,
}
