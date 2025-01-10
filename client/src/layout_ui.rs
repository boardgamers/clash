use crate::hex_ui::Point;
use crate::tooltip;
use macroquad::color::WHITE;
use macroquad::math::{f32, vec2, Vec2};
use macroquad::prelude::*;
use crate::render_context::RenderContext;

pub const ICON_SIZE: f32 = 30.;

pub const MARGIN: f32 = 10.;

pub const FONT_SIZE: u16 = 20;

pub fn icon_offset(i: i8) -> f32 {
    f32::from(i) * 1.4 * ICON_SIZE
}

pub fn icon_pos(x: i8, y: i8) -> Vec2 {
    vec2(icon_offset(x), icon_offset(y))
}

pub fn top_center_texture(rc: &RenderContext, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = top_center_anchor(rc);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub fn top_centered_text(rc: &RenderContext, text: &str, p: Vec2) {
    let p = top_center_anchor(rc) + p;
    rc.state.draw_text(text, p.x - rc.state.measure_text(text).width / 2., p.y);
}

fn top_center_anchor(rc: &RenderContext) -> Vec2 {
    vec2(rc.state.screen_size.x / 2., MARGIN)
}

pub fn top_right_texture(rc: &RenderContext, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(rc.state.screen_size.x - MARGIN, MARGIN);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub fn bottom_left_texture(rc: &RenderContext, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(MARGIN, rc.state.screen_size.y - MARGIN);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub fn bottom_center_texture(rc: &RenderContext, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = bottom_center_anchor(rc);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub fn bottom_centered_text(rc: &RenderContext, text: &str) {
    let dimensions = rc.state.measure_text(text);
    bottom_center_text(rc, text, vec2(-dimensions.width / 2., -50.));
}

pub fn bottom_center_text(rc: &RenderContext, text: &str, p: Vec2) {
    let p = bottom_center_anchor(rc) + p;
    rc.state.draw_text(text, p.x, p.y);
}

pub fn bottom_center_anchor(rc: &RenderContext) -> Vec2 {
    vec2(rc.state.screen_size.x / 2., rc.state.screen_size.y - MARGIN)
}

pub fn bottom_right_texture(rc: &RenderContext, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(rc.state.screen_size.x - MARGIN, rc.state.screen_size.y - MARGIN);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub fn draw_icon(rc: &RenderContext, texture: &Texture2D, tooltip: &str, origin: Vec2) -> bool {
    draw_scaled_icon(rc, texture, tooltip, origin, ICON_SIZE)
}

pub fn draw_scaled_icon(
    rc: &RenderContext,
    texture: &Texture2D,
    tooltip: &str,
    origin: Vec2,
    size: f32,
) -> bool {
    draw_texture_ex(
        texture,
        origin.x,
        origin.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            ..Default::default()
        },
    );

    let rect = Rect::new(origin.x, origin.y, size, size);
    if !tooltip.is_empty() {
        tooltip::show_tooltip_for_rect(rc, &[tooltip.to_string()], rect);
    }
    left_mouse_button_pressed_in_rect(rect, rc)
}

#[must_use]
pub fn left_mouse_button_pressed_in_rect(rect: Rect, rc: &RenderContext) -> bool {
    left_mouse_button_pressed(rc).is_some_and(|p| rect.contains(p))
}

#[must_use]
pub fn is_in_circle(mouse_pos: Vec2, p: Point, radius: f32) -> bool {
    let d = vec2(p.x - mouse_pos.x, p.y - mouse_pos.y);
    d.length() <= radius
}

#[must_use]
pub fn left_mouse_button_pressed(rc: &RenderContext) -> Option<Vec2> {
    if is_mouse_button_pressed(MouseButton::Left) {
        Some(rc.screen_to_world(mouse_position().into()))
    } else {
        None
    }
}
