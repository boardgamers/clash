pub enum Unit {
    Ship,
    ArmyUnit(ArmyUnit),
    Settler,
}

pub enum ArmyUnit {
    Infantry,
    Cavalry,
    Elephant,
    Leader,
}
