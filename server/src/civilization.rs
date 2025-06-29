use crate::content::civilizations::{BARBARIANS, CHOOSE_CIV, PIRATES};
use crate::map::Block;
use crate::{leader::LeaderInfo, special_advance::SpecialAdvanceInfo};

#[derive(Clone)]
pub struct Civilization {
    pub name: String,
    pub special_advances: Vec<SpecialAdvanceInfo>,
    pub leaders: Vec<LeaderInfo>,
    pub(crate) start_block: Option<Block>,
}

impl Civilization {
    #[must_use]
    pub fn new(
        name: &str,
        special_advances: Vec<SpecialAdvanceInfo>,
        leaders: Vec<LeaderInfo>,
        start_block: Option<Block>,
    ) -> Self {
        Self {
            name: name.to_string(),
            special_advances,
            leaders,
            start_block,
        }
    }

    #[must_use]
    pub fn is_barbarian(&self) -> bool {
        self.name == BARBARIANS
    }

    #[must_use]
    pub fn is_pirates(&self) -> bool {
        self.name == PIRATES
    }

    #[must_use]
    pub fn is_choose_civ(&self) -> bool {
        self.name == CHOOSE_CIV
    }

    #[must_use]
    pub fn is_human(&self) -> bool {
        !self.is_barbarian() && !self.is_pirates()
    }

    #[must_use]
    pub fn can_choose(&self) -> bool {
        self.is_human() || !self.is_choose_civ()
    }
}
