use crate::{city::City, landmark::Landmark};

pub struct Hexagon {
    pub position: HexagonPosition,
    pub city: Option<City>,
    pub landmark: Landmark,
    pub discovered: bool,
}

impl Hexagon {
    pub fn new(position: HexagonPosition, landmark: Landmark) -> Hexagon {
        Hexagon {
            position,
            city: None,
            landmark,
            discovered: false,
        }
    }
}

pub struct HexagonPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl HexagonPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Self) -> u32 {
        let distance_x = self.x.abs_diff(other.x);
        let distance_y = self.y.abs_diff(other.y);
        let distance_z = self.z.abs_diff(other.z);
        distance_x + distance_y + distance_z
    }
}

pub struct Tile {
    hexagons: Vec<Hexagon>,
}
