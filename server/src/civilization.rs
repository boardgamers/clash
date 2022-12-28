use crate::{hexagon::Tile, leader::Leader, special_technology::SpecialTechnology};

pub struct Civilization {
    pub name: String,
    pub special_technologies: Vec<SpecialTechnology>,
    pub leaders: Vec<Leader>,
    pub special_tile: Option<Tile>,
}

impl Civilization {
    pub fn new(
        name: &str,
        special_technologies: Vec<SpecialTechnology>,
        leaders: Vec<Leader>,
        special_tile: Option<Tile>,
    ) -> Self {
        Self {
            name: name.to_string(),
            special_technologies,
            leaders,
            special_tile,
        }
    }
}
