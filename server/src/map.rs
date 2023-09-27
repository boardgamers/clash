use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::position::Position;

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
    pub fn cloned_data(&self) -> MapData {
        MapData {
            tiles: self
                .tiles
                .clone()
                .into_iter()
                .sorted_by_key(|(position, _)| *position)
                .collect(),
        }
    }

    #[must_use]
    pub fn from_data(data: MapData) -> Self {
        Self {
            tiles: data.tiles.into_iter().collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MapData {
    pub tiles: Vec<(Position, Terrain)>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Terrain {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Exhausted(Box<Terrain>),
    Water,
}
