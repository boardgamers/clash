use crate::client_state::{ShownPlayer, State, ZOOM};
use macroquad::color::WHITE;
use macroquad::math::{vec2, Vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;

pub fn top_left_label(p: Vec2, label: &str) {
    root_ui().label(p, label);
}

pub fn top_center_label(player: &ShownPlayer, p: Vec2, label: &str) {
    root_ui().label(vec2(player.screen_size.x / 2.0, 0.) + p, label);
}
//
// pub fn top_center_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
//     relative_texture(state, texture, vec2(screen_width() / 2.0, 0.), p)
// }

pub fn top_right_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(state, texture, vec2(screen_width(), 0.), p)
}

pub fn bottom_left_texture(state: &State, texture: &Texture2D, p: Vec2) -> bool {
    relative_texture(state, texture, vec2(0., screen_height()), p)
}

fn relative_texture(state: &State, texture: &Texture2D, anchor: Vec2, offset: Vec2) -> bool {
    let size = screen_width() / 30. * ZOOM / state.zoom;
    let origin = anchor + offset * (screen_width() / 100.); // * s;
    let world = state.camera.screen_to_world(origin);
    draw_texture_ex(
        texture,
        world.x,
        world.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            ..Default::default()
        },
    );

    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        Rect::new(origin.x, origin.y, size, size).contains(vec2(x, y))
    } else {
        false
    }
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

pub fn bottom_left_button(player: &ShownPlayer, p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(0., player.screen_size.y) + p, label)
}

pub fn bottom_right_button(player: &ShownPlayer, p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(player.screen_size.x, player.screen_size.y) + p, label)
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
