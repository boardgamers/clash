use crate::client::Features;
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
    pub async fn new(features: &Features) -> Self {
        Self {
            terrain: Self::terrain(features).await,
            units: HashMap::new(),
            // cities: HashMap::new(),
            // resources: HashMap::new(),
        }
    }

    async fn terrain(features: &Features) -> HashMap<Terrain, Texture2D> {
        let mut map: HashMap<Terrain, Texture2D> = HashMap::new();

        for (t, f) in [
            (Terrain::Barren, "barren.png"),
            (Terrain::Mountain, "mountain.png"),
            (Terrain::Fertile, "grassland.png"),
            (Terrain::Forest, "forest.png"),
            (Terrain::Water, "water.png"),
        ] {
            let url = &features.assets_url;
            map.insert(t, load_texture(&format!("{url}{f}")).await.unwrap());
        }
        map
    }
}
