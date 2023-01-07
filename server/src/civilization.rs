use crate::{leader::Leader, special_technology::SpecialTechnology};

pub struct Civilization {
    pub name: String,
    pub special_technologies: Vec<SpecialTechnology>,
    pub leaders: Vec<Leader>,
}

impl Civilization {
    pub fn new(
        name: &str,
        special_technologies: Vec<SpecialTechnology>,
        leaders: Vec<Leader>,
    ) -> Self {
        Self {
            name: name.to_string(),
            special_technologies,
            leaders,
        }
    }
}
