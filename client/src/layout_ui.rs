use crate::client_state::{ShownPlayer, State};
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
    relative_texture(
        state,
        texture,
        vec2(state.screen_size.x / 2., MARGIN),
        p,
        tooltip,
    )
}

pub fn top_right_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    relative_texture(
        state,
        texture,
        vec2(state.screen_size.x - MARGIN, MARGIN),
        p,
        tooltip,
    )
}

pub fn bottom_left_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    relative_texture(
        state,
        texture,
        vec2(MARGIN, state.screen_size.y - MARGIN),
        p,
        tooltip,
    )
}

pub fn bottom_center_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    relative_texture(
        state,
        texture,
        bottom_center_anchor(state),
        p,
        tooltip,
    )
}

pub fn bottom_center_anchor(state: &State) -> Vec2 {
    vec2(state.screen_size.x / 2., state.screen_size.y - MARGIN)
}

pub fn bottom_right_texture(state: &State, texture: &Texture2D, p: Vec2, tooltip: &str) -> bool {
    relative_texture(
        state,
        texture,
        vec2(state.screen_size.x - MARGIN, state.screen_size.y - MARGIN),
        p,
        tooltip,
    )
}

fn relative_texture(
    state: &State,
    texture: &Texture2D,
    anchor: Vec2,
    offset: Vec2,
    tooltip: &str,
) -> bool {
    let origin = anchor + offset;

    draw_texture_ex(
        texture,
        origin.x,
        origin.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(ICON_SIZE, ICON_SIZE)),
            ..Default::default()
        },
    );

    let rect = Rect::new(origin.x, origin.y, ICON_SIZE, ICON_SIZE);
    tooltip::show_tooltip_for_rect(state, tooltip, rect);
    left_mouse_button(rect)
}

pub fn left_mouse_button(rect: Rect) -> bool {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        rect.contains(vec2(x, y))
    } else {
        false
    }
}

pub fn cancel_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(player.screen_size.x / 4.0, 190.))
        .unwrap_or_else(|| Vec2::new(player.screen_size.x / 2., player.screen_size.y - 130.))
}

pub fn ok_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(player.screen_size.x / 4.0 - 150., 190.))
        .unwrap_or_else(|| {
            Vec2::new(
                player.screen_size.x / 2. - 150.,
                player.screen_size.y - 130.,
            )
        })
}

pub fn ok_only_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(player.screen_size.x / 4.0 - 75., 190.))
        .unwrap_or_else(|| Vec2::new(player.screen_size.x / 2. - 75., player.screen_size.y - 130.))
}

fn small_dialog(player: &ShownPlayer) -> bool {
    player.active_dialog.is_map_dialog() || player.pending_update
}
