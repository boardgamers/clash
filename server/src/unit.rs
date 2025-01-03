use std::iter;
use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};

use serde::{Deserialize, Serialize};

use MovementRestriction::{AllMovement, Attack};
use UnitType::*;

use crate::{game::Game, map::Terrain::*, position::Position, resource_pile::ResourcePile, utils};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Unit {
    pub player_index: usize,
    pub position: Position,
    pub unit_type: UnitType,
    pub movement_restriction: MovementRestriction,
    pub id: u32,
}

impl Unit {
    #[must_use]
    pub fn new(player_index: usize, position: Position, unit_type: UnitType, id: u32) -> Self {
        Self {
            player_index,
            position,
            unit_type,
            movement_restriction: MovementRestriction::None,
            id,
        }
    }

    #[must_use]
    pub fn can_move(&self) -> bool {
        !matches!(self.movement_restriction, AllMovement(_))
    }

    #[must_use]
    pub fn can_attack(&self) -> bool {
        matches!(self.movement_restriction, MovementRestriction::None)
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if unit is at a valid position
    #[must_use]
    pub fn can_found_city(&self, game: &Game) -> bool {
        if !self.unit_type.is_settler() {
            return false;
        }
        if self.is_transported(game) {
            return false;
        }
        let player = &game.players[self.player_index];
        if player.get_city(self.position).is_some() {
            return false;
        }
        if matches!(
            game.map
                .tiles
                .get(&self.position)
                .expect("The unit should be at a valid position"),
            Barren | Exhausted(_)
        ) {
            return false;
        }
        if player.available_settlements == 0 {
            return false;
        }
        true
    }

    #[must_use]
    pub fn is_transported(&self, game: &Game) -> bool {
        self.unit_type.is_land_based() && game.map.tiles[&self.position] == Water
    }

    pub fn restrict_movement(&mut self) {
        self.movement_restriction = match self.movement_restriction {
            MovementRestriction::None => AllMovement(0),
            AllMovement(x) | Attack(x) => AllMovement(x),
        }
    }

    pub fn undo_movement_restriction(&mut self) {
        self.movement_restriction = match self.movement_restriction {
            MovementRestriction::None | AllMovement(0) => MovementRestriction::None,
            AllMovement(x) | Attack(x) => Attack(x),
        }
    }

    pub fn restrict_attack(&mut self) {
        self.movement_restriction = match self.movement_restriction {
            MovementRestriction::None => Attack(1),
            AllMovement(x) => AllMovement(x),
            Attack(x) => Attack(x + 1),
        }
    }

    pub fn undo_attack_restriction(&mut self) {
        self.movement_restriction = match self.movement_restriction {
            AllMovement(x) => AllMovement(x),
            Attack(1) | MovementRestriction::None => MovementRestriction::None,
            Attack(x) => Attack(x - 1),
        }
    }

    pub fn reset_movement_restriction(&mut self) {
        self.movement_restriction = MovementRestriction::None;
    }
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug)]
pub enum UnitType {
    Settler,
    Infantry,
    Ship,
    Cavalry,
    Elephant,
    Leader,
}

impl UnitType {
    #[must_use]
    pub fn cost(&self) -> ResourcePile {
        match self {
            Settler | Elephant => ResourcePile::food(2),
            Infantry => ResourcePile::food(1) + ResourcePile::ore(1),
            Ship => ResourcePile::wood(2),
            Cavalry => ResourcePile::food(1) + ResourcePile::wood(1),
            Leader => ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1),
        }
    }

    #[must_use]
    pub fn is_land_based(&self) -> bool {
        !matches!(self, Ship)
    }

    #[must_use]
    pub fn is_army_unit(&self) -> bool {
        matches!(self, Infantry | Cavalry | Elephant | Leader)
    }

    /// Returns `true` if the unit type is [`Settler`].
    ///
    /// [`Settler`]: UnitType::Settler
    #[must_use]
    pub fn is_settler(&self) -> bool {
        matches!(self, Self::Settler)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum MovementRestriction {
    None,
    AllMovement(u32),
    Attack(u32),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Units {
    pub settlers: u8,
    pub infantry: u8,
    pub ships: u8,
    pub cavalry: u8,
    pub elephants: u8,
    pub leaders: u8,
}

impl Units {
    #[must_use]
    pub fn new(
        settlers: u8,
        infantry: u8,
        ships: u8,
        cavalry: u8,
        elephants: u8,
        leaders: u8,
    ) -> Self {
        Self {
            settlers,
            infantry,
            ships,
            cavalry,
            elephants,
            leaders,
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub fn has_unit(&self, unit: &UnitType) -> bool {
        self.get(unit) > 0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clone().to_vec().is_empty()
    }

    #[must_use]
    pub fn get(&self, unit: &UnitType) -> u8 {
        match *unit {
            Settler => self.settlers,
            Infantry => self.infantry,
            Ship => self.ships,
            Cavalry => self.cavalry,
            Elephant => self.elephants,
            Leader => self.leaders,
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, unit: &UnitType) -> &mut u8 {
        match *unit {
            Settler => &mut self.settlers,
            Infantry => &mut self.infantry,
            Ship => &mut self.ships,
            Cavalry => &mut self.cavalry,
            Elephant => &mut self.elephants,
            Leader => &mut self.leaders,
        }
    }

    #[must_use]
    pub fn to_vec(self) -> Vec<UnitType> {
        self.into_iter()
            .flat_map(|(u, c)| iter::repeat(u).take(c as usize))
            .collect()
    }

    #[must_use]
    pub fn get_units_to_replace(&self, new_units: &Units) -> Units {
        let mut units_to_replace = Units::empty();
        for (unit_type, count) in self.clone() {
            let new_count = new_units.get(&unit_type) as i8;
            let replace = new_count - count as i8;
            if replace > 0 {
                *units_to_replace.get_mut(&unit_type) += replace as u8;
            }
        }
        units_to_replace
    }
}

impl AddAssign<&UnitType> for Units {
    fn add_assign(&mut self, rhs: &UnitType) {
        match *rhs {
            Settler => self.settlers += 1,
            Infantry => self.infantry += 1,
            Ship => self.ships += 1,
            Cavalry => self.cavalry += 1,
            Elephant => self.elephants += 1,
            Leader => self.leaders += 1,
        };
    }
}

impl SubAssign<&UnitType> for Units {
    fn sub_assign(&mut self, rhs: &UnitType) {
        match *rhs {
            Settler => self.settlers -= 1,
            Infantry => self.infantry -= 1,
            Ship => self.ships -= 1,
            Cavalry => self.cavalry -= 1,
            Elephant => self.elephants -= 1,
            Leader => self.leaders -= 1,
        };
    }
}

impl FromIterator<UnitType> for Units {
    fn from_iter<T: IntoIterator<Item = UnitType>>(iter: T) -> Self {
        let mut units = Self::empty();
        for unit in iter {
            units += &unit;
        }
        units
    }
}

impl IntoIterator for Units {
    type Item = (UnitType, u8);
    type IntoIter = UnitsIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        UnitsIntoIterator {
            units: self,
            index: 0,
        }
    }
}

pub struct UnitsIntoIterator {
    units: Units,
    index: u8,
}

impl Iterator for UnitsIntoIterator {
    type Item = (UnitType, u8);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        let u = &self.units;
        match index {
            0 => Some((Settler.clone(), u.settlers)),
            1 => Some((Infantry.clone(), u.infantry)),
            2 => Some((Ship.clone(), u.ships)),
            3 => Some((Cavalry.clone(), u.cavalry)),
            4 => Some((Elephant.clone(), u.elephants)),
            5 => Some((Leader.clone(), u.leaders)),
            _ => None,
        }
    }
}

impl Display for Units {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut unit_types = Vec::new();
        if self.settlers > 0 {
            unit_types.push(format!(
                "{} {}",
                self.settlers,
                if self.settlers == 1 {
                    "settler"
                } else {
                    "settlers"
                }
            ));
        }
        if self.infantry > 0 {
            unit_types.push(format!("{} infantry", self.infantry,));
        }
        if self.ships > 0 {
            unit_types.push(format!(
                "{} {}",
                self.ships,
                if self.ships == 1 { "ship" } else { "ships" }
            ));
        }
        if self.cavalry > 0 {
            unit_types.push(format!("{} cavalry", self.cavalry,));
        }
        if self.elephants > 0 {
            unit_types.push(format!(
                "{} {}",
                self.elephants,
                if self.elephants == 1 {
                    "elephant"
                } else {
                    "elephants"
                }
            ));
        }
        if self.leaders > 0 {
            unit_types.push(if self.leaders == 1 {
                String::from("a leader")
            } else {
                format!("{} leaders", self.leaders)
            });
        }
        write!(f, "{}", utils::format_list(&unit_types, "no units"))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum MovementAction {
    Move {
        units: Vec<u32>,
        destination: Position,
    },
    Stop,
}

#[cfg(test)]
mod tests {
    use crate::unit::UnitType::*;
    use crate::unit::Units;

    #[test]
    fn into_iter() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        assert_eq!(
            units.into_iter().collect::<Vec<_>>(),
            vec![
                (Settler, 0),
                (Infantry, 1),
                (Ship, 0),
                (Cavalry, 2),
                (Elephant, 1),
                (Leader, 1),
            ]
        );
    }

    #[test]
    fn to_vec() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        assert_eq!(
            units.to_vec(),
            vec![Infantry, Cavalry, Cavalry, Elephant, Leader]
        );
    }

    #[test]
    fn get_units_to_replace() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        let new_units = Units::new(0, 2, 0, 1, 1, 1);
        assert_eq!(
            units.get_units_to_replace(&new_units),
            Units::new(0, 1, 0, 0, 0, 0)
        );
    }
}
