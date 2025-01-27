use crate::client::Features;
use macroquad::prelude::{load_texture, load_ttf_font, Font, ImageFormat};
use macroquad::texture::Texture2D;
use server::city_pieces::Building;
use server::content::custom_actions::CustomActionType;
use server::map::Terrain;
use server::resource::ResourceType;
use server::unit::UnitType;
use std::collections::HashMap;

pub struct Assets {
    pub terrain: HashMap<Terrain, Texture2D>,
    pub exhausted: Texture2D,
    pub units: HashMap<UnitType, Texture2D>,
    pub font: Font,

    // mood icons
    pub angry: Texture2D,

    // action icons
    pub move_units: Texture2D,
    pub log: Texture2D,
    pub end_turn: Texture2D,
    pub advances: Texture2D,
    pub rotate_explore: Texture2D,

    // UI
    pub redo: Texture2D,
    pub reset: Texture2D,
    pub undo: Texture2D,
    pub plus: Texture2D,
    pub minus: Texture2D,
    pub ok_blocked: Texture2D,
    pub ok: Texture2D,
    pub cancel: Texture2D,

    pub victory_points: Texture2D,
    pub active_player: Texture2D,

    // Admin
    pub import: Texture2D,
    pub export: Texture2D,

    pub resources: HashMap<ResourceType, Texture2D>,
    pub buildings: HashMap<Building, Texture2D>,
    pub wonders: HashMap<String, Texture2D>,
    pub custom_actions: HashMap<CustomActionType, Texture2D>,
}

impl Assets {
    pub async fn new(features: &Features) -> Self {
        let font_name = features.get_asset("HTOWERT.TTF");
        Self {
            font: load_ttf_font(&font_name).await.unwrap(), // can't share font - causes panic
            terrain: Self::terrain(features).await,
            exhausted: load_png(include_bytes!("../assets/cross-svgrepo-com.png")),
            units: Self::units(),

            angry: load_png(include_bytes!("../assets/angry-face-svgrepo-com.png")),
            resources: Self::resources(),
            buildings: Self::buildings(),
            wonders: Self::wonders(),
            custom_actions: Self::custom_actions(),

            // action icons
            advances: load_png(include_bytes!("../assets/lab-svgrepo-com.png")),
            end_turn: load_png(include_bytes!("../assets/hour-glass-svgrepo-com.png")),
            log: load_png(include_bytes!("../assets/scroll-svgrepo-com.png")),
            move_units: load_png(include_bytes!("../assets/route-start-svgrepo-com.png")),
            rotate_explore: load_png(include_bytes!("../assets/rotate-svgrepo-com.png")),

            // UI
            redo: load_png(include_bytes!("../assets/redo-svgrepo-com.png")),
            reset: load_png(include_bytes!("../assets/reset-svgrepo-com.png")),
            undo: load_png(include_bytes!("../assets/undo-svgrepo-com.png")),
            plus: load_png(include_bytes!("../assets/plus-circle-svgrepo-com.png")),
            minus: load_png(include_bytes!("../assets/minus-circle-svgrepo-com.png")),
            ok: load_png(include_bytes!("../assets/ok-circle-svgrepo-com.png")),
            ok_blocked: load_png(include_bytes!("../assets/in-progress-svgrepo-com.png")),
            cancel: load_png(include_bytes!("../assets/cancel-svgrepo-com.png")),

            victory_points: load_png(include_bytes!("../assets/trophy-cup-svgrepo-com.png")),
            active_player: load_png(include_bytes!("../assets/triangle-svgrepo-com.png")),

            // Admin
            import: load_png(include_bytes!("../assets/import-3-svgrepo-com.png")),
            export: load_png(include_bytes!("../assets/export-2-svgrepo-com.png")),
        }
    }

    fn wonders() -> HashMap<String, Texture2D> {
        [
            (
                "Pyramids".to_string(),
                load_png(include_bytes!("../assets/pyramid-svgrepo-com.png")),
            ),
            (
                "Great Gardens".to_string(),
                // todo find a better icon
                load_png(include_bytes!("../assets/pyramid-svgrepo-com.png")),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    fn units() -> HashMap<UnitType, Texture2D> {
        [
            (
                UnitType::Infantry,
                load_png(include_bytes!("../assets/warrior-svgrepo-com.png")),
            ),
            (
                UnitType::Settler,
                load_png(include_bytes!("../assets/wagon-svgrepo-com.png")),
            ),
            (
                UnitType::Cavalry,
                load_png(include_bytes!("../assets/horse-head-svgrepo-com.png")),
            ),
            (
                UnitType::Elephant,
                load_png(include_bytes!("../assets/elephant-svgrepo-com.png")),
            ),
            (
                UnitType::Ship,
                load_png(include_bytes!("../assets/ship-svgrepo-com.png")),
            ),
            (
                UnitType::Leader,
                load_png(include_bytes!("../assets/flag-svgrepo-com.png")),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    fn resources() -> HashMap<ResourceType, Texture2D> {
        [
            (
                ResourceType::Food,
                load_png(include_bytes!("../assets/wheat-grain-svgrepo-com.png")),
            ),
            (
                ResourceType::Wood,
                load_png(include_bytes!("../assets/wood-nature-svgrepo-com.png")),
            ),
            (
                ResourceType::Ore,
                load_png(include_bytes!("../assets/rock-svgrepo-com.png")),
            ),
            (
                ResourceType::Ideas,
                load_png(include_bytes!("../assets/light-bulb-idea-svgrepo-com.png")),
            ),
            (
                ResourceType::Gold,
                load_png(include_bytes!("../assets/gold-ingots-gold-svgrepo-com.png")),
            ),
            (
                ResourceType::MoodTokens,
                load_png(include_bytes!("../assets/happy-emoji-svgrepo-com.png")),
            ),
            (
                ResourceType::CultureTokens,
                load_png(include_bytes!("../assets/theater-drama-svgrepo-com.png")),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    fn buildings() -> HashMap<Building, Texture2D> {
        [
            (
                Building::Academy,
                load_png(include_bytes!("../assets/academy-cap-svgrepo-com.png")),
            ),
            (
                Building::Market,
                load_png(include_bytes!("../assets/market-place-svgrepo-com.png")),
            ),
            (
                Building::Obelisk,
                load_png(include_bytes!("../assets/obelisk-svgrepo-com.png")),
            ),
            (
                Building::Observatory,
                load_png(include_bytes!(
                    "../assets/observatory-exploration-svgrepo-com.png"
                )),
            ),
            (
                Building::Fortress,
                load_png(include_bytes!(
                    "../assets/castle-fortress-14-svgrepo-com.png"
                )),
            ),
            (
                Building::Port,
                load_png(include_bytes!("../assets/port-location-svgrepo-com.png")),
            ),
            (
                Building::Temple,
                load_png(include_bytes!(
                    "../assets/temple-building-with-columns-svgrepo-com.png"
                )),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    fn custom_actions() -> HashMap<CustomActionType, Texture2D> {
        [(
            CustomActionType::ForcedLabor,
            load_png(include_bytes!("../assets/slavery-whip-svgrepo-com.png")),
        )]
        .iter()
        .cloned()
        .collect()
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
            map.insert(t, load_texture(&features.get_asset(f)).await.unwrap());
        }
        map
    }
}

fn load_png(bytes: &[u8]) -> Texture2D {
    Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png))
}
