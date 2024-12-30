use crate::client::Features;
use crate::resource_ui::ResourceType;
use macroquad::prelude::{load_texture, load_ttf_font, Color, Image, ImageFormat, RectOffset};
use macroquad::texture::Texture2D;
use macroquad::ui::{root_ui, Skin};
use server::map::Terrain;
use server::unit::UnitType;
use std::collections::HashMap;

pub struct Assets {
    pub terrain: HashMap<Terrain, Texture2D>,
    pub units: HashMap<UnitType, Texture2D>,
    pub skin: Skin,

    // mood icons
    pub angry: Texture2D,
    pub neutral: Texture2D,
    pub happy: Texture2D,

    // action icons
    pub movement: Texture2D,
    pub log: Texture2D,
    pub end_turn: Texture2D,
    pub advances: Texture2D,
    pub redo: Texture2D,
    pub reset: Texture2D,
    pub undo: Texture2D,

    // UI
    pub zoom_in: Texture2D,
    pub zoom_out: Texture2D,
    pub up: Texture2D,
    pub down: Texture2D,
    pub left: Texture2D,
    pub right: Texture2D,
    pub victory_points: Texture2D,

    // Admin
    pub import: Texture2D,
    pub export: Texture2D,

    // pub cities: HashMap<CityType, Texture2D>,
    pub resources: HashMap<ResourceType, Texture2D>,
}

impl Assets {
    pub async fn new(features: &Features) -> Self {
        let happy = load_png(include_bytes!("../assets/happy-emoji-svgrepo-com.png"));
        Self {
            terrain: Self::terrain(features).await,
            units: HashMap::new(),
            skin: Self::skin(features).await,

            // mood icons
            angry: load_png(include_bytes!("../assets/angry-face-svgrepo-com.png")),
            neutral: load_png(include_bytes!("../assets/neutral-face-svgrepo-com.png")),
            happy: happy.clone(),

            // resource icons
            resources: [
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
                (ResourceType::MoodTokens, happy.clone()),
                (
                    ResourceType::CultureTokens,
                    load_png(include_bytes!("../assets/theater-drama-svgrepo-com.png")),
                ),
            ]
            .iter()
            .cloned()
            .collect(),

            // action icons
            advances: load_png(include_bytes!("../assets/lab-svgrepo-com.png")),
            end_turn: load_png(include_bytes!("../assets/hour-glass-svgrepo-com.png")),
            log: load_png(include_bytes!("../assets/scroll-svgrepo-com.png")),
            movement: load_png(include_bytes!("../assets/walk-svgrepo-com.png")),
            redo: load_png(include_bytes!("../assets/redo-svgrepo-com.png")),
            reset: load_png(include_bytes!("../assets/reset-svgrepo-com.png")),
            undo: load_png(include_bytes!("../assets/undo-svgrepo-com.png")),

            // UI
            zoom_in: load_png(include_bytes!("../assets/zoom-in-1462-svgrepo-com.png")),
            zoom_out: load_png(include_bytes!("../assets/zoom-out-1460-svgrepo-com.png")),
            up: load_png(include_bytes!("../assets/up-arrow-circle-svgrepo-com.png")),
            down: load_png(include_bytes!(
                "../assets/down-arrow-circle-svgrepo-com.png"
            )),
            left: load_png(include_bytes!(
                "../assets/left-arrow-circle-svgrepo-com.png"
            )),
            right: load_png(include_bytes!(
                "../assets/right-arrow-circle-svgrepo-com.png"
            )),
            victory_points: load_png(include_bytes!("../assets/trophy-cup-svgrepo-com.png")),

            // Admin
            import: load_png(include_bytes!("../assets/import-3-svgrepo-com.png")),
            export: load_png(include_bytes!("../assets/export-2-svgrepo-com.png")),
            // cities: HashMap::new(),
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
            map.insert(t, load_texture(&features.get_asset(f)).await.unwrap());
        }
        map
    }

    async fn skin(features: &Features) -> Skin {
        let font = load_ttf_font(&features.get_asset("HTOWERT.TTF"))
            .await
            .unwrap();
        let image =
            Image::from_file_with_format(include_bytes!("../assets/button_background.png"), None)
                .unwrap();
        let label_style = root_ui()
            .style_builder()
            .background(image.clone())
            .background_margin(RectOffset::new(37.0, 37.0, 5.0, 5.0))
            .margin(RectOffset::new(10.0, 10.0, 0.0, 0.0))
            .with_font(&font)
            .unwrap()
            .text_color(Color::from_rgba(180, 180, 120, 255))
            .font_size(20)
            .build();

        let window_style = root_ui()
            .style_builder()
            .background(
                Image::from_file_with_format(
                    include_bytes!("../assets/window_background.png"),
                    None,
                )
                .unwrap(),
            )
            .background_margin(RectOffset::new(20.0, 20.0, 10.0, 10.0))
            .margin(RectOffset::new(-20.0, -30.0, 0.0, 0.0))
            .build();

        let button_style = root_ui()
            .style_builder()
            .background(image)
            .background_margin(RectOffset::new(37.0, 37.0, 5.0, 5.0))
            .margin(RectOffset::new(10.0, 10.0, 0.0, 0.0))
            .background_hovered(
                Image::from_file_with_format(
                    include_bytes!("../assets/button_hovered_background.png"),
                    None,
                )
                .unwrap(),
            )
            .background_clicked(
                Image::from_file_with_format(
                    include_bytes!("../assets/button_clicked_background.png"),
                    None,
                )
                .unwrap(),
            )
            .with_font(&font)
            .unwrap()
            .text_color(Color::from_rgba(180, 180, 100, 255))
            .font_size(20)
            .build();

        let editbox_style = root_ui()
            .style_builder()
            .background_margin(RectOffset::new(0., 0., 0., 0.))
            .with_font(&font)
            .unwrap()
            .text_color(Color::from_rgba(120, 120, 120, 255))
            .color_selected(Color::from_rgba(190, 190, 190, 255))
            .font_size(50)
            .build();

        // let checkbox_style = root_ui()
        //     .style_builder()
        //     .background(
        //         Image::from_file_with_format(
        //             include_bytes!("../examples/ui_assets/checkbox_background.png"),
        //             None,
        //         )
        //         .unwrap(),
        //     )
        //     .background_hovered(
        //         Image::from_file_with_format(
        //             include_bytes!("../examples/ui_assets/checkbox_hovered_background.png"),
        //             None,
        //         )
        //         .unwrap(),
        //     )
        //     .background_clicked(
        //         Image::from_file_with_format(
        //             include_bytes!("../examples/ui_assets/checkbox_clicked_background.png"),
        //             None,
        //         )
        //         .unwrap(),
        //     )
        //     .build();

        Skin {
            editbox_style,
            window_style,
            button_style,
            window_titlebar_style: label_style.clone(),
            label_style,
            // checkbox_style,
            title_height: 30.,
            ..root_ui().default_skin()
        }
    }
}

fn load_png(bytes: &[u8]) -> Texture2D {
    Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png))
}
