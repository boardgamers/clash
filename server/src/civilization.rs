use crate::{leader::Leader, special_advance::SpecialAdvance};

//todo! add optional special starting tile
pub struct Civilization {
    pub name: String,
    pub special_advances: Vec<SpecialAdvance>,
    pub leaders: Vec<Leader>,
}

impl Civilization {
    pub fn new(name: &str, special_advances: Vec<SpecialAdvance>, leaders: Vec<Leader>) -> Self {
        Self {
            name: name.to_string(),
            special_advances,
            leaders,
        }
    }
}
