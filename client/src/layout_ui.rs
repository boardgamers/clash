use crate::client_state::{ShownPlayer, State};
use macroquad::color::WHITE;
use macroquad::math::{vec2, Vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;

pub const ICON_SIZE: f32 = 30.;

pub const MARGIN: f32 = 10.;

pub fn icon_offset(i: i8) -> f32 {
    f32::from(i) * 1.4 * ICON_SIZE
}

pub fn icon_pos(x: i8, y: i8) -> Vec2 {
    vec2(icon_offset(x), icon_offset(y))
}

pub fn top_left_label(p: Vec2, label: &str) {
    root_ui().label(p, label);
}

pub fn top_center_label(player: &ShownPlayer, p: Vec2, label: &str) {
    root_ui().label(vec2(player.screen_size.x / 2.0, 0.) + p, label);
}

pub fn top_center_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(state, texture, vec2(state.screen_size.x / 2., MARGIN), p)
}

pub fn top_right_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(
        state,
        texture,
        vec2(state.screen_size.x - MARGIN, MARGIN),
        p,
    )
}

pub fn bottom_left_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(
        state,
        texture,
        vec2(MARGIN, state.screen_size.y - MARGIN),
        p,
    )
}

pub fn bottom_left_button(player: &ShownPlayer, p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(MARGIN, player.screen_size.y - MARGIN) + p, label)
}

pub fn bottom_right_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(
        state,
        texture,
        vec2(state.screen_size.x - MARGIN, state.screen_size.y - MARGIN),
        p,
    )
}

fn relative_texture(state: &State, texture: &Texture2D, anchor: Vec2, offset: Vec2) -> bool {
    let origin = anchor + offset;
    set_default_camera();
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

    let pressed = if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        Rect::new(origin.x, origin.y, ICON_SIZE, ICON_SIZE).contains(vec2(x, y))
    } else {
        false
    };
    set_camera(&state.camera);
    pressed
}

pub fn right_center_label(player: &ShownPlayer, p: Vec2, label: &str) {
    root_ui().label(
        vec2(player.screen_size.x, player.screen_size.y / 2.0) + p,
        label,
    );
}

pub fn right_center_button(player: &ShownPlayer, p: Vec2, label: &str) -> bool {
    root_ui().button(
        vec2(player.screen_size.x, player.screen_size.y / 2.0) + p,
        label,
    )
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
