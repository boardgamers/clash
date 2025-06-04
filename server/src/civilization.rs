use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::{leader::LeaderInfo, special_advance::SpecialAdvanceInfo};

#[derive(Clone)]
pub struct Civilization {
    pub name: String,
    pub special_advances: Vec<SpecialAdvanceInfo>,
    pub leaders: Vec<LeaderInfo>,
}

impl Civilization {
    pub fn new(
        name: &str,
        special_advances: Vec<SpecialAdvanceInfo>,
        leaders: Vec<LeaderInfo>,
    ) -> Self {
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
