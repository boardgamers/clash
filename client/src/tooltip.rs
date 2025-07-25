use crate::client_state::{CameraMode, MousePosition, NO_UPDATE, State};
use crate::layout_ui::rect_from;
use crate::log_ui::MultilineText;
use crate::render_context::RenderContext;
use macroquad::color::{Color, GRAY};
use macroquad::input::mouse_position;
use macroquad::math::{Rect, Vec2, bool, f32, f64, vec2};
use macroquad::prelude::{draw_circle, draw_rectangle, get_time};

const TOOLTIP_DELAY: f64 = 0.5;

pub(crate) fn update(state: &mut State) {
    let now = get_time();
    state
        .mouse_positions
        .retain(|mp| now - mp.time < TOOLTIP_DELAY);
    state.mouse_positions.push(MousePosition {
        position: mouse_position().into(),
        time: now,
    });
}

fn is_rect_tooltip_active(rc: &RenderContext, rect: Rect) -> bool {
    rc.stage.is_tooltip()
        && rc
            .state
            .mouse_positions
            .iter()
            .all(|mp| rect.contains(rc.screen_to_world(mp.position)))
}

pub(crate) fn show_tooltip_for_rect(
    rc: &RenderContext,
    tooltip: &MultilineText,
    rect: Rect,
    right_offset: f32,
) {
    let origin = rect.point();
    if is_rect_tooltip_active(rc, rect) {
        draw_rectangle(
            origin.x,
            origin.y,
            rect.size().x,
            rect.size().y,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        let _ = rc.with_camera(CameraMode::Screen, false, |rc| {
            let rect = rect_from(
                rc.world_to_screen(rect.point()),
                rc.world_to_screen(rect.size()),
            );
            show_tooltip_text(rc, tooltip, rect.point(), right_offset);
            NO_UPDATE
        });
    }
}

fn is_circle_tooltip_active(rc: &RenderContext, center: Vec2, radius: f32) -> bool {
    rc.stage.is_tooltip()
        && rc
            .state
            .mouse_positions
            .iter()
            .all(|mp| (center - rc.screen_to_world(mp.position)).length() < radius)
}

pub(crate) fn show_tooltip_for_circle(
    rc: &RenderContext,
    tooltip: &MultilineText,
    center: Vec2,
    radius: f32,
) {
    if is_circle_tooltip_active(rc, center, radius) {
        draw_circle(center.x, center.y, radius, Color::new(0.0, 0.0, 0.0, 0.5));
        let _ = rc.with_camera(CameraMode::Screen, false, |rc| {
            let center = rc.world_to_screen(center);
            show_tooltip_text(rc, tooltip, center + vec2(radius, radius), 50.);
            NO_UPDATE
        });
    }
}

fn show_tooltip_text(rc: &RenderContext, tooltip: &MultilineText, origin: Vec2, right_offset: f32) {
    let state = rc.state;
    let dim = tooltip.text.iter().map(|t| state.measure_text(t));
    let total = dim.fold(Vec2::new(0., 0.), |acc, d| {
        vec2(acc.x.max(d.width), acc.y + 20.)
    });

    let tooltip_rect = rect_from(origin, total);
    let w = tooltip_rect.size().x + 10.;
    let sx = state.screen_size.x - right_offset;
    let x = tooltip_rect.left().min(sx - w);
    let y = (tooltip_rect.top() - 10.)
        .max(40.)
        .min(state.screen_size.y - tooltip_rect.h - 40.);
    draw_rectangle(x, y, w, tooltip_rect.size().y + 10., GRAY);
    for (i, line) in tooltip.text.iter().enumerate() {
        state.draw_text(line, x + 5., y + 20. + i as f32 * 20.);
    }
}
