use macroquad::prelude::{load_texture, Texture2D};
use server::map::Terrain;
use server::unit::UnitType;
use std::collections::HashMap;

pub struct Assets {
    pub units: HashMap<UnitType, Texture2D>,
    // pub cities: HashMap<CityType, Texture2D>,
    // pub resources: HashMap<Resource, Texture2D>,
}

impl Assets {
    pub async fn new() -> Self {
        Self {
            units: HashMap::new(),
            // cities: HashMap::new(),
            // resources: HashMap::new(),
        }
    }
}
