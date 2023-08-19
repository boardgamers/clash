use serde::{Deserialize, Serialize};

use crate::{position::Position, resource_pile::ResourcePile};

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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum UnitType {
    Settler,
    Infantry,
    Ship,
    Cavalry,
    Elephant,
    Leader,
}

impl UnitType {
    pub fn cost(&self) -> ResourcePile {
        match self {
            Settler | Elephant => ResourcePile::food(2),
            Infantry => ResourcePile::food(1) + ResourcePile::ore(1),
            Ship => ResourcePile::wood(2),
            Cavalry => ResourcePile::food(1) + ResourcePile::wood(1),
            Leader => ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1),
        }
    }

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
