use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::{leader::Leader, special_advance::SpecialAdvanceInfo};

//todo add optional special starting tile
#[derive(Clone)]
pub struct Civilization {
    pub name: String,
    pub special_advances: Vec<SpecialAdvanceInfo>,
    pub leaders: Vec<Leader>,
}

impl Civilization {
    pub fn new(name: &str, special_advances: Vec<SpecialAdvanceInfo>, leaders: Vec<Leader>) -> Self {
        Self {
            name: name.to_string(),
            special_advances,
            leaders,
        }
    }

    pub fn is_barbarian(&self) -> bool {
        self.name == BARBARIANS
    }

    pub fn is_pirates(&self) -> bool {
        self.name == PIRATES
    }

    pub fn is_human(&self) -> bool {
        !self.is_barbarian() && !self.is_pirates()
    }
}
