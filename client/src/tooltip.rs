use crate::client_state::{MousePosition, State};
use macroquad::camera::set_default_camera;
use macroquad::color::{Color, GRAY};
use macroquad::input::mouse_position;
use macroquad::math::{bool, f32, f64, vec2, Rect, Vec2};
use macroquad::prelude::{draw_circle, draw_rectangle, get_time};

const TOOLTIP_DELAY: f64 = 0.5;

pub fn update(state: &mut State) {
    let (x, y) = mouse_position();
    let now = get_time();
    state
        .mouse_positions
        .retain(|mp| now - mp.time < TOOLTIP_DELAY);
    state.mouse_positions.push(MousePosition {
        position: vec2(x, y),
        time: now,
    });
}

fn is_rect_tooltip_active(state: &State, rect: Rect) -> bool {
    state
        .mouse_positions
        .iter()
        .all(|mp| rect.contains(state.screen_to_world(mp.position)))
}

pub fn show_tooltip_for_rect(state: &State, tooltip: &[String], rect: Rect) {
    let origin = rect.point();
    let screen_origin = state.world_to_screen(rect.point());
    if is_rect_tooltip_active(state, rect) {
        draw_rectangle(
            origin.x,
            origin.y,
            rect.size().x,
            rect.size().y,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        set_default_camera();
        show_tooltip_text(state, tooltip, screen_origin);
        state.set_camera();
    }
}

fn is_circle_tooltip_active(state: &State, center: Vec2, radius: f32) -> bool {
    state
        .mouse_positions
        .iter()
        .all(|mp| (center - state.screen_to_world(mp.position)).length() < radius)
}

pub fn show_tooltip_for_circle(state: &State, tooltip: &str, center: Vec2, radius: f32) {
    let screen_center = state.world_to_screen(center);
    if is_circle_tooltip_active(state, center, radius) {
        draw_circle(center.x, center.y, radius, Color::new(0.0, 0.0, 0.0, 0.5));
        set_default_camera();
        show_tooltip_text(
            state,
            &[tooltip.to_string()],
            screen_center + vec2(radius, radius),
        );
        state.set_camera();
    }
}

fn show_tooltip_text(state: &State, tooltip: &[String], origin: Vec2) {
    let dim = tooltip.iter().map(|t| state.measure_text(t));
    let total = dim.fold(Vec2::new(0., 0.), |acc, d| {
        vec2(acc.x.max(d.width), acc.y + 20.)
    });

    let tooltip_rect = Rect::new(origin.x, origin.y, total.x, total.y);
    let w = tooltip_rect.size().x + 10.;
    let sx = state.screen_size.x;
    let x = tooltip_rect.left().min(sx - w);
    let y = (tooltip_rect.top() - 10.).max(40.);
    draw_rectangle(x, y, w, tooltip_rect.size().y + 10., GRAY);
    for (i, line) in tooltip.iter().enumerate() {
        state.draw_text(line, x + 5., y + 20. + i as f32 * 20.);
    }
}
