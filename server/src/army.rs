use serde::{Deserialize, Serialize};

use crate::hexagon::HexagonPosition;

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub player: String,
    pub unit_type: UnitType,
    pub transporter: Option<HexagonPosition>,
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
