use crate::client_state::ShownPlayer;
use macroquad::math::{vec2, Vec2};
use macroquad::ui::root_ui;

pub fn top_left_label(p: Vec2, label: &str) {
    root_ui().label(p, label);
}

pub fn top_center_label(player: &ShownPlayer, p: Vec2, label: &str) {
    root_ui().label(vec2(player.screen_size.x / 2.0, 0.) + p, label);
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
