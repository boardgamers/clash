use crate::client_state::{MousePosition, ShownPlayer, State};
use macroquad::color::WHITE;
use macroquad::math::{vec2, Vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;

pub const ICON_SIZE: f32 = 30.;

pub const MARGIN: f32 = 10.;

pub const TOOLTIP_DELAY: f64 = 0.5;

pub fn icon_offset(i: i8) -> f32 {
    f32::from(i) * 1.4 * ICON_SIZE
}

pub fn icon_pos(x: i8, y: i8) -> Vec2 {
    vec2(icon_offset(x), icon_offset(y))
}

pub fn top_left_label(p: Vec2, label: &str) {
    root_ui().label(p + vec2(-40., 0.), label);
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
    show_tooltip(state, tooltip, rect);
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

pub fn update_tooltip(state: &mut State) {
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

fn is_active_tooltip(state: &State, rect: Rect) -> bool {
    state
        .mouse_positions
        .iter()
        .all(|mp| rect.contains(mp.position))
}

pub fn show_tooltip(state: &State, tooltip: &str, rect: Rect) {
    let origin = rect.point();
    if is_active_tooltip(state, rect) {
        draw_rectangle(
            origin.x,
            origin.y,
            rect.size().x,
            rect.size().y,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        let dimensions = draw_tooltip_text(tooltip, origin, BLANK);
        let tooltip_rect = Rect::new(origin.x, origin.y, dimensions.width, dimensions.height);
        let w = tooltip_rect.size().x + 10.;
        let sx = state.screen_size.x;
        let x = tooltip_rect.left().min(sx - w);
        let y = (tooltip_rect.top() - 10.).max(40.);
        draw_rectangle(
            x,
            y,
            w,
            tooltip_rect.size().y + 10.,
            GREEN,
        );
        draw_tooltip_text(tooltip, vec2(x, y + 10.), BLACK);
    }
}

fn draw_tooltip_text(tooltip: &str, origin: Vec2, color: Color) -> TextDimensions {
    draw_text(
        tooltip,
        origin.x + 5.,
        origin.y + 5.,
        20.,
        color,
    )
}
