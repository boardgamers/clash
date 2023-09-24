use macroquad::prelude::{load_texture, Texture2D};
use server::map::Terrain;
use server::unit::UnitType;
use std::collections::HashMap;

pub struct Assets {
    pub terrain: HashMap<Terrain, Texture2D>,
    pub units: HashMap<UnitType, Texture2D>,
    // pub cities: HashMap<CityType, Texture2D>,
    // pub resources: HashMap<Resource, Texture2D>,
}

impl Assets {
    pub async fn new() -> Self {
        Self {
            terrain: Self::terrain().await,
            units: HashMap::new(),
            // cities: HashMap::new(),
            // resources: HashMap::new(),
        }
    }

    async fn terrain() -> HashMap<Terrain, Texture2D> {
        let mut map: HashMap<Terrain, Texture2D> = HashMap::new();

        for (t, f) in [
            (Terrain::Barren, "assets/barren.png"),
            (Terrain::Mountain, "assets/mountain.png"),
            (Terrain::Fertile, "assets/grassland.png"),
            (Terrain::Forest, "assets/forest.png"),
            (Terrain::Water, "assets/water.png"),
        ] {
            map.insert(t, load_texture(f).await.unwrap());
        }

        map
    }
}
