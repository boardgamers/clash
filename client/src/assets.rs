use crate::client::Features;
use macroquad::prelude::{Font, ImageFormat, load_texture, load_ttf_font};
use macroquad::texture::Texture2D;
use server::city_pieces::Building;
use server::content::custom_actions::CustomActionType;
use server::map::Terrain;
use server::player::Player;
use server::resource::ResourceType;
use server::unit::UnitType;
use std::collections::HashMap;

pub struct CivAssets {
    pub units: HashMap<UnitType, Texture2D>,
}

pub struct Assets {
    pub terrain: HashMap<Terrain, Texture2D>,
    pub exhausted: Texture2D,
    pub font: Font,

    // mood icons
    pub angry: Texture2D,

    // action icons
    pub move_units: Texture2D,
    pub log: Texture2D,
    pub show_permanent_effects: Texture2D,
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
    pub event_counter: Texture2D,
    pub active_player: Texture2D,

    // Admin
    pub import: Texture2D,
    pub export: Texture2D,
    pub play: Texture2D,
    pub pause: Texture2D,

    pub resources: HashMap<ResourceType, Texture2D>,
    pub buildings: HashMap<Building, Texture2D>,
    pub wonders: HashMap<String, Texture2D>,
    pub custom_actions: HashMap<CustomActionType, Texture2D>,
    pub civ: HashMap<String, CivAssets>,
    pub default_civ: CivAssets,
}

impl Assets {
    pub async fn new(features: &Features) -> Self {
        let font_name = features.get_asset("HTOWERT.TTF");
        Self {
            font: load_ttf_font(&font_name).await.unwrap(), // can't share font - causes panic
            terrain: Self::terrain(features).await,
            exhausted: load_png(include_bytes!("../assets/cross-svgrepo-com.png")),

            angry: load_png(include_bytes!("../assets/angry-face-svgrepo-com.png")),
            resources: Self::resources(),
            buildings: Self::buildings(),
            civ: Self::new_civ_assets(),
            default_civ: Self::new_default_civ(),
            wonders: Self::wonders(),
            custom_actions: Self::custom_actions(),

            // action icons
            advances: load_png(include_bytes!("../assets/lab-svgrepo-com.png")),
            end_turn: load_png(include_bytes!("../assets/hour-glass-svgrepo-com.png")),
            log: load_png(include_bytes!("../assets/scroll-svgrepo-com.png")),
            show_permanent_effects: load_png(include_bytes!("../assets/trojan.png")),
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
            event_counter: load_png(include_bytes!("../assets/counter-clockwise-clock-svgrepo-com.png")),
            active_player: load_png(include_bytes!("../assets/triangle-svgrepo-com.png")),

            // Admin
            import: load_png(include_bytes!("../assets/import-3-svgrepo-com.png")),
            export: load_png(include_bytes!("../assets/export-2-svgrepo-com.png")),
            play: load_png(include_bytes!("../assets/play-1003-svgrepo-com.png")),
            pause: load_png(include_bytes!("../assets/pause-1006-svgrepo-com.png")),
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

    fn units() -> Vec<(UnitType, Option<String>, Texture2D)> {
        vec![
            (
                UnitType::Infantry,
                None,
                load_png(include_bytes!("../assets/warrior-svgrepo-com.png")),
            ),
            (
                UnitType::Settler,
                None,
                load_png(include_bytes!("../assets/wagon-svgrepo-com.png")),
            ),
            (
                UnitType::Cavalry,
                None,
                load_png(include_bytes!("../assets/horse-head-svgrepo-com.png")),
            ),
            (
                UnitType::Elephant,
                None,
                load_png(include_bytes!("../assets/elephant-svgrepo-com.png")),
            ),
            (
                UnitType::Ship,
                None,
                load_png(include_bytes!("../assets/ship-svgrepo-com.png")),
            ),
            (
                UnitType::Ship,
                Some("Pirates".to_string()),
                load_png(include_bytes!(
                    "../assets/pirate-symbol-mark-svgrepo-com.png"
                )),
            ),
            (
                UnitType::Leader,
                None,
                load_png(include_bytes!("../assets/flag-svgrepo-com.png")),
            ),
        ]
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
        [
            (
                CustomActionType::AbsolutePower,
                load_png(include_bytes!("../assets/crown-svgrepo-com.png")),
            ),
            (
                CustomActionType::ForcedLabor,
                load_png(include_bytes!("../assets/slavery-whip-svgrepo-com.png")),
            ),
            (
                CustomActionType::CivilLiberties,
                load_png(include_bytes!("../assets/justice-hammer-svgrepo-com.png")),
            ),
            (
                CustomActionType::Taxes,
                load_png(include_bytes!("../assets/tax-svgrepo-com.png")),
            ),
            (
                CustomActionType::Theaters,
                load_png(include_bytes!(
                    "../assets/temple-building-with-columns-svgrepo-com.png"
                )),
            ),
            (
                CustomActionType::Sports,
                load_png(include_bytes!("../assets/stadium-svgrepo-com.png")),
            ),
        ]
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

    fn new_civ_assets() -> HashMap<String, CivAssets> {
        let mut result = HashMap::new();
        for (unit, civ, t) in Self::units() {
            if let Some(civ) = civ {
                result
                    .entry(civ)
                    .or_insert_with(|| CivAssets {
                        units: HashMap::new(),
                    })
                    .units
                    .insert(unit, t);
            }
        }
        result
    }

    fn new_default_civ() -> CivAssets {
        let mut units = HashMap::new();
        for (unit, civ, t) in Self::units() {
            if civ.is_none() {
                units.insert(unit, t);
            }
        }
        CivAssets { units }
    }

    pub fn unit(&self, unit_type: UnitType, player: &Player) -> &Texture2D {
        self.civ
            .get(&player.civilization.name)
            .and_then(|c| c.units.get(&unit_type))
            .unwrap_or(&self.default_civ.units[&unit_type])
    }
}

fn load_png(bytes: &[u8]) -> Texture2D {
    Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png))
}
