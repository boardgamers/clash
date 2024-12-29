use crate::client::Features;
use crate::resource_ui::ResourceType;
use crate::svg;
use macroquad::prelude::{load_texture, load_ttf_font, Color, Image, RectOffset};
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

    // action icons
    pub movement: Texture2D,
    pub log: Texture2D,
    pub hour_glass: Texture2D,
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

    // Admin
    pub import: Texture2D,
    pub export: Texture2D,

    // pub cities: HashMap<CityType, Texture2D>,
    pub resources: HashMap<ResourceType, Texture2D>,
}

impl Assets {
    pub async fn new(features: &Features) -> Self {
        Self {
            terrain: Self::terrain(features).await,
            units: HashMap::new(),
            skin: Self::skin(features).await,

            // mood icons
            angry: svg::icon(include_str!("../assets/angry.svg")),
            neutral: svg::icon(include_str!("../assets/neutral.svg")),

            // resource icons
            resources: [
                (
                    ResourceType::Food,
                    svg::icon(include_str!("../assets/food.svg")),
                ),
                (
                    ResourceType::Wood,
                    svg::icon(include_str!("../assets/wood.svg")),
                ),
                (
                    ResourceType::Ore,
                    svg::icon(include_str!("../assets/rock.svg")),
                ),
                (
                    ResourceType::Ideas,
                    svg::icon(include_str!("../assets/idea.svg")),
                ),
                (
                    ResourceType::Gold,
                    svg::icon(include_str!("../assets/gold.svg")),
                ),
                (
                    ResourceType::MoodTokens,
                    svg::icon(include_str!("../assets/happy.svg")),
                ),
                (
                    ResourceType::CultureTokens,
                    svg::icon(include_str!("../assets/culture.svg")),
                ),
            ]
            .iter()
            .cloned()
            .collect(),

            // action icons
            advances: svg::icon(include_str!("../assets/lab.svg")),
            hour_glass: svg::icon(include_str!("../assets/hour-glass.svg")),
            log: svg::icon(include_str!("../assets/scroll.svg")),
            movement: svg::icon(include_str!("../assets/move.svg")),
            redo: svg::icon(include_str!("../assets/redo.svg")),
            reset: svg::icon(include_str!("../assets/reset.svg")),
            undo: svg::icon(include_str!("../assets/undo.svg")),

            // UI
            zoom_in: svg::icon(include_str!("../assets/zoom-in.svg")),
            zoom_out: svg::icon(include_str!("../assets/zoom-out.svg")),
            up: svg::icon(include_str!("../assets/up-arrow.svg")),
            down: svg::icon(include_str!("../assets/down-arrow.svg")),
            left: svg::icon(include_str!("../assets/left-arrow.svg")),
            right: svg::icon(include_str!("../assets/right-arrow.svg")),

            // Admin
            import: svg::icon(include_str!("../assets/import.svg")),
            export: svg::icon(include_str!("../assets/export.svg")),
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
