use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Unit {
    Ship,
    ArmyUnit(ArmyUnit),
    Settler,
}

#[derive(Serialize, Deserialize)]
pub enum ArmyUnit {
    Infantry,
    Cavalry,
    Elephant,
    Leader,
}
