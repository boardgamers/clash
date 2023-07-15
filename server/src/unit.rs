use serde::{Deserialize, Serialize};

use crate::position::Position;

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub player_index: usize,
    pub unit_type: UnitType,
    pub transporter: Option<Position>,
    pub can_attack: bool,
}

#[derive(Serialize, Deserialize)]
pub enum UnitType {
    Ship,
    ArmyUnit(ArmyUnitType),
    Settler,
}

#[derive(Serialize, Deserialize)]
pub enum ArmyUnitType {
    Infantry,
    Cavalry,
    Elephant,
    Leader,
}
