use hex2d::Coordinate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Copy)]
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
        let row = chars.next().expect("string is emtpy") as u32 - 'A' as u32;
        let col = s.get(1..).expect("string is too short").parse::<u32>().expect("not a number");
        let q = (col - 1) as i32;
        let r = (row as i32) - (q - (q % 2)) / 2;
        Position::new (q, r)
    }

    pub fn name(&self) -> String {
        let c = self.q;
        let r = self.r + (self.q - (self.q % 2)) / 2;
        let row = char::from_u32(('A' as u32) + r as u32).unwrap();
        let col = c + 1;
        format!("{row}{col}")
    }

    pub fn coordinate(&self) -> Coordinate {
        // x == r
        // y == q
        Coordinate::new(self.r, self.q)
    }

    pub fn distance(&self, other: &Self) -> u32 {
        self.coordinate().distance(other.coordinate()) as u32
    }
}

impl ToString for Position {
    fn to_string(&self) -> String {
        self.name()
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
        let position = Position::from_offset("A1");
        assert_eq!(Position::new(0, 0), position);
        assert_eq!("A1", position.name());
    }
}
