use std::fmt::Display;

use hex2d::Coordinate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Position {
    pub q: i32,
    pub r: i32,
}

impl Position {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    // using Odd Q - https://www.redblobgames.com/grids/hexagons/#coordinates-offset
    pub fn from_offset(s: &str) -> Position {
        let mut chars = s.chars();
        let col = chars.next().expect("string is emtpy") as u32 - 'A' as u32;
        let row = s
            .get(1..)
            .expect("string is too short")
            .parse::<u32>()
            .expect("not a number")
            - 1;
        let q = col as i32;
        let r = (row as i32) - (q - (q.rem_euclid(2))) / 2;
        Position::new(q, r)
    }

    pub fn coordinate(&self) -> Coordinate {
        // x == q
        // y == r
        Coordinate::new(self.q, self.r)
    }

    pub fn distance(&self, other: &Self) -> u32 {
        self.coordinate().distance(other.coordinate()) as u32
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let row = (self.r + (self.q - (self.q.rem_euclid(2))) / 2) + 1;
        let col = char::from_u32(('A' as u32) + self.q as u32).unwrap();
        write!(f, "{col}{row}")
    }
}

pub enum Landmark {
    Barren,
    Mountain,
    Fertile,
    Forest,
    Unusable,
    Water,
}

#[cfg(test)]
mod tests {
    use crate::hexagon::Position;

    #[test]
    fn convert_position() {
        assert_eq!(Position::new(2, -1), Position::from_offset("C1"));
        assert_eq!(Position::new(0, 0), Position::from_offset("A1"));
        assert_eq!(Position::new(1, 2), Position::from_offset("B3"));
        assert_inverse("B1");
        assert_inverse("A1");
        assert_inverse("B2");
        assert_inverse("B5");
    }

    fn assert_inverse(offset: &str) {
        assert_eq!(offset, Position::from_offset(offset).to_string());
    }
}
