use crate::client_state::State;
use crate::hex_ui::Point;
use crate::tooltip;
use macroquad::color::WHITE;
use macroquad::math::{f32, vec2, Vec2};
use macroquad::prelude::*;

pub const ICON_SIZE: f32 = 30.;

pub const MARGIN: f32 = 10.;

pub const FONT_SIZE: u16 = 20;

pub fn icon_offset(i: i8) -> f32 {
    f32::from(i) * 1.4 * ICON_SIZE
}

pub fn icon_pos(x: i8, y: i8) -> Vec2 {
    vec2(icon_offset(x), icon_offset(y))
}

pub fn top_center_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = top_center_anchor(state);
    draw_icon(state, texture, tooltip, anchor + p)
}

pub fn top_center_text(state: &State, text: &str, p: Vec2) {
    let p = top_center_anchor(state) + p;
    state.draw_text(text, p.x, p.y);
}

fn top_center_anchor(state: &State) -> Vec2 {
    vec2(state.screen_size.x / 2., MARGIN)
}

pub fn top_right_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(state.screen_size.x - MARGIN, MARGIN);
    draw_icon(state, texture, tooltip, anchor + p)
}

pub fn bottom_left_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(MARGIN, state.screen_size.y - MARGIN);
    draw_icon(state, texture, tooltip, anchor + p)
}

pub fn bottom_center_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = bottom_center_anchor(state);
    draw_icon(state, texture, tooltip, anchor + p)
}

pub fn bottom_center_text(state: &State, text: &str, p: Vec2) {
    let p = bottom_center_anchor(state) + p;
    state.draw_text(text, p.x, p.y);
}

pub fn bottom_center_anchor(state: &State) -> Vec2 {
    vec2(state.screen_size.x / 2., state.screen_size.y - MARGIN)
}

pub fn bottom_right_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    let anchor = vec2(state.screen_size.x - MARGIN, state.screen_size.y - MARGIN);
    draw_icon(state, texture, tooltip, anchor + p)
}

pub fn draw_icon(state: &State, texture: &Texture2D, tooltip: &str, origin: Vec2) -> bool {
    draw_scaled_icon(state, texture, tooltip, origin, ICON_SIZE)
}

pub fn draw_scaled_icon(
    state: &State,
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
        tooltip::show_tooltip_for_rect(state, &[tooltip.to_string()], rect);
    }
    left_mouse_button_pressed_in_rect(rect, state)
}

#[must_use]
pub fn left_mouse_button_pressed_in_rect(rect: Rect, state: &State) -> bool {
    left_mouse_button_pressed(state).is_some_and(|p| rect.contains(p))
}

#[must_use]
pub fn is_in_circle(mouse_pos: Vec2, p: Point, radius: f32) -> bool {
    let d = vec2(p.x - mouse_pos.x, p.y - mouse_pos.y);
    d.length() <= radius
}

#[must_use]
pub fn left_mouse_button_pressed(state: &State) -> Option<Vec2> {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        Some(state.screen_to_world(vec2(x, y)))
    } else {
        None
    }
}
