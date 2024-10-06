use crate::client_state::ShownPlayer;
use macroquad::math::{vec2, Vec2};
use macroquad::prelude::screen_height;
use macroquad::ui::root_ui;
use macroquad::window::screen_width;

pub fn top_left_label(p: Vec2, label: &str) {
    root_ui().label(p, label);
}

pub fn top_center_label(p: Vec2, label: &str) {
    root_ui().label(vec2(screen_width() / 2.0, 0.) + p, label);
}

pub fn right_center_label(p: Vec2, label: &str) {
    root_ui().label(vec2(screen_width(), screen_height() / 2.0) + p, label);
}

pub fn right_center_button(p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(screen_width(), screen_height() / 2.0) + p, label)
}

pub fn bottom_left_button(p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(0., screen_height()) + p, label)
}

pub fn bottom_right_button(p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(screen_width(), screen_height()) + p, label)
}

pub fn cancel_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(screen_width() / 4.0, 190.))
        .unwrap_or_else(|| Vec2::new(screen_width() / 2., screen_height() - 130.))
}

pub fn ok_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(screen_width() / 4.0 - 150., 190.))
        .unwrap_or_else(|| Vec2::new(screen_width() / 2. - 150., screen_height() - 130.))
}

pub fn ok_only_pos(player: &ShownPlayer) -> Vec2 {
    small_dialog(player)
        .then(|| Vec2::new(screen_width() / 4.0 - 75., 190.))
        .unwrap_or_else(|| Vec2::new(screen_width() / 2. - 75., screen_height() - 130.))
}

fn small_dialog(player: &ShownPlayer) -> bool {
    player.active_dialog.is_map_dialog() || player.pending_update
}
