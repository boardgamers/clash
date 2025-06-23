use hex2d::Coordinate;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(PartialEq, Eq, Copy, Clone, Hash)]
pub struct Position {
    pub q: i32,
    pub r: i32,
}

impl Position {
    #[must_use]
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if an invalid position format is given
    /// using Odd Q - <https://www.redblobgames.com/grids/hexagons/#coordinates-offset>
    #[must_use]
    pub fn from_offset(s: &str) -> Position {
        let mut chars = s.chars();
        let col = chars.next().expect("string is empty") as i32 - 'A' as i32;
        let row = s
            .get(1..)
            .expect("string is too short")
            .parse::<i32>()
            .expect("not a number")
            - 1;
        let q = col;
        let r = row - (q - (q.rem_euclid(2))) / 2;
        Position::new(q, r)
    }

    #[must_use]
    pub fn coordinate(&self) -> Coordinate {
        // x == q
        // y == r
        Coordinate::new(self.q, self.r)
    }

    #[must_use]
    pub fn from_coordinate(coordinate: Coordinate) -> Position {
        Position::new(coordinate.x, coordinate.y)
    }

    #[must_use]
    pub fn distance(&self, other: Self) -> u32 {
        self.coordinate().distance(other.coordinate()) as u32
    }

    #[must_use]
    pub fn is_neighbor(&self, other: Self) -> bool {
        self.coordinate().distance(other.coordinate()) == 1
    }

    pub fn neighbors(&self) -> Vec<Self> {
        self.coordinate()
            .neighbors()
            .into_iter()
            .map(Position::from_coordinate)
            .collect()
    }

    #[must_use]
    pub fn next_position_in_path(&self, target: &Self) -> Option<Self> {
        if self == target {
            return None;
        }
        let mut neighbors = self.neighbors();
        neighbors.sort_by_key(|a| a.distance(*target));
        if neighbors.is_empty() {
            None
        } else {
            Some(neighbors[0])
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let row = (self.r + (self.q - (self.q.rem_euclid(2))) / 2) + 1;
        let col = char::from_u32(('A' as u32) + self.q as u32)
            .unwrap_or_else(|| panic!("Invalid column index: {}{}", self.q, self.r));
        write!(f, "{col}{row}")
    }
}

impl Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Serialize for Position {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Position::from_offset(&s))
    }
}

#[cfg(test)]
mod tests {
    use crate::position::Position;

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
