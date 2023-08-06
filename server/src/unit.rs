use serde::{Deserialize, Serialize};

use crate::position::Position;

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub player_index: usize,
    pub position: Position,
    pub unit_type: UnitType,
    pub movement_restriction: MovementRestriction,
    pub transporter: Option<Position>,
}

impl Unit {
    pub fn new(player_index: usize, position: Position, unit_type: UnitType) -> Self {
        Self {
            player_index,
            position,
            unit_type,
            movement_restriction: MovementRestriction::None,
            transporter: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum UnitType {
    Infantry,
    Cavalry,
    Elephant,
    Leader,
    Ship,
    Settler,
}

#[derive(Serialize, Deserialize)]
pub enum MovementRestriction {
    None,
    Attack,
    AllMovement,
}
