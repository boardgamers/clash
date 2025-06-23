use std::f32::consts::PI;

use crate::layout_ui::draw_scaled_icon;
use crate::render_context::RenderContext;
use hex2d::{Coordinate, Spacing};
use macroquad::color::Color;
use macroquad::math::{Vec2, f32, vec2};
use macroquad::prelude::{
    BEIGE, DARKGRAY, DrawTextureParams, Rect, Texture2D, WHITE, draw_texture_ex,
};
use macroquad::shapes::draw_hexagon;
use server::content::custom_actions::CustomActionType;
use server::map::block_for_position;
use server::position::Position;

const SIZE: f32 = 60.0;

const SHORT_SIZE: f32 = SIZE * 0.866_025_4;

const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: Position) -> Vec2 {
    let c = pos.coordinate();
    let p = c.to_pixel(SPACING);
    to_screen(Vec2::new(p.0, p.1))
}

pub enum HexFeature {
    Exhausted,
    ExplorerToken,
}

pub fn draw_hex(
    p: Position,
    text_color: Color,
    overlay: Color,
    terrain: Option<&Texture2D>,
    features: &[HexFeature],
    rc: &RenderContext,
) {
    let c = center(p);
    if rc.stage.is_main() {
        if let Some(terrain) = terrain {
            draw_texture_ex(
                terrain,
                c.x - SIZE,
                c.y - SHORT_SIZE,
                WHITE,
                DrawTextureParams {
                    source: Some(Rect::new(0., 0., 298., 257.)),
                    dest_size: Some(vec2(SIZE * 2.0, SHORT_SIZE * 2.)),
                    ..Default::default()
                },
            );
        } else {
            // Terrain::Unexplored
            draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, BEIGE);
        }
        draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, overlay);
        let text_y = c.y - 35.0;
        rc.draw_text_with_color(&p.to_string(), c.x - 30.0, text_y, text_color);
        rc.draw_text_with_color(
            &block_for_position(rc.game, p).0.to_string(),
            c.x + 10.0,
            text_y,
            text_color,
        );
    }

    for f in features {
        match f {
            HexFeature::Exhausted => {
                const SIZE: f32 = 100.;
                draw_scaled_icon(
                    rc,
                    &rc.assets().exhausted,
                    "Exhausted",
                    vec2(c.x - SIZE / 2., c.y - SIZE / 2.),
                    SIZE,
                );
            }
            HexFeature::ExplorerToken => {
                const SIZE: f32 = 20.;
                draw_scaled_icon(
                    rc,
                    &rc.assets().custom_actions[&CustomActionType::LegendaryExplorer],
                    "Explorer Token",
                    vec2(c.x - SIZE / 2., c.y - SIZE / 2.),
                    SIZE,
                );
            }
        }
    }
}

pub fn pixel_to_coordinate(p: Vec2) -> Coordinate {
    let p = to_game(Vec2::new(p.x, p.y));
    Coordinate::from_pixel(p.x, p.y, SPACING)
}

pub fn rotate_around(center: Vec2, radius: f32, angle_deg: usize) -> Vec2 {
    rotate_around_rad(center, radius, PI / 180.0 * (angle_deg as f32))
}

pub fn rotate_around_rad(center: Vec2, radius: f32, angle_rad: f32) -> Vec2 {
    Vec2::new(
        center.x + radius * f32::cos(angle_rad),
        center.y + radius * f32::sin(angle_rad),
    )
}

pub fn to_screen(p: Vec2) -> Vec2 {
    let x = p.x + LEFT_BORDER;
    let y = TOP_BORDER - p.y;
    Vec2::new(x, y)
}

pub fn to_game(p: Vec2) -> Vec2 {
    let x = p.x - LEFT_BORDER;
    let y = TOP_BORDER - p.y;
    Vec2::new(x, y)
}

const TOP_BORDER: f32 = 0.0;
const LEFT_BORDER: f32 = 0.0;
