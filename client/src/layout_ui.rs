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

pub fn bottom_left_button(p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(0., screen_height()) + p, label)
}

pub fn bottom_right_button(p: Vec2, label: &str) -> bool {
    root_ui().button(vec2(screen_width(), screen_height()) + p, label)
}
