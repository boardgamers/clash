use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};

use serde::{Deserialize, Serialize};

use crate::{position::Position, resource_pile::ResourcePile, utils};

use UnitType::*;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Unit {
    pub player_index: usize,
    pub position: Position,
    pub unit_type: UnitType,
    pub movement_restriction: MovementRestriction,
    pub transporter_position: Option<Position>,
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
            transporter_position: None,
            id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
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
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum MovementRestriction {
    None,
    Attack,
    AllMovement,
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
        match *unit {
            Settler => self.settlers > 0,
            Infantry => self.infantry > 0,
            Ship => self.ships > 0,
            Cavalry => self.cavalry > 0,
            Elephant => self.elephants > 0,
            Leader => self.leaders > 0,
        }
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
