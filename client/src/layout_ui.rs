use crate::log_ui::break_text;
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_rect;
use macroquad::color::WHITE;
use macroquad::math::{Vec2, f32, vec2};
use macroquad::prelude::*;

pub const ICON_SIZE: f32 = 30.;

pub const MARGIN: f32 = 10.;

pub const FONT_SIZE: u16 = 20;

pub const UI_BACKGROUND: Color = WHITE.with_alpha(0.8);

pub(crate) fn icon_offset(i: i8) -> f32 {
    f32::from(i) * 1.4 * ICON_SIZE
}

pub(crate) fn icon_pos(x: i8, y: i8) -> Vec2 {
    vec2(icon_offset(x), icon_offset(y))
}

pub(crate) fn top_center_texture(
    rc: &RenderContext,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) -> bool {
    draw_icon(rc, texture, tooltip, top_center_anchor(rc) + p)
}

pub(crate) fn top_centered_text(rc: &RenderContext, text: &str, p: Vec2) {
    let p = top_center_anchor(rc) + p;
    rc.draw_text(text, p.x - rc.state.measure_text(text).width / 2., p.y);
}

pub(crate) fn top_center_anchor(rc: &RenderContext) -> Vec2 {
    vec2(rc.state.screen_size.x / 2., MARGIN)
}

pub(crate) fn top_right_texture(
    rc: &RenderContext,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) -> bool {
    let anchor = vec2(rc.state.screen_size.x - MARGIN, MARGIN);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub(crate) fn bottom_left_texture(
    rc: &RenderContext,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &[String],
) -> bool {
    let anchor = vec2(MARGIN, rc.state.screen_size.y - MARGIN);
    draw_scaled_icon_with_tooltip(rc, texture, tooltip, anchor + p, ICON_SIZE)
}

pub(crate) fn bottom_center_texture(
    rc: &RenderContext,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) -> bool {
    let anchor = bottom_center_anchor(rc);
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub(crate) fn bottom_centered_text_with_offset(
    rc: &RenderContext,
    text: &str,
    offset: Vec2,
    tooltip: &[String],
) {
    let dimensions = rc.state.measure_text(text);
    let p = vec2(-dimensions.width / 2., -50.) + offset;
    let a = bottom_center_anchor(rc) + p;
    let rect = Rect::new(
        a.x,
        a.y - dimensions.offset_y,
        dimensions.width,
        dimensions.height,
    );
    rc.draw_rectangle(rect, UI_BACKGROUND);
    bottom_center_text(rc, text, p);
    if !tooltip.is_empty() {
        show_tooltip_for_rect(rc, tooltip, rect, 50.);
    }
}

pub(crate) fn bottom_centered_text(rc: &RenderContext, text: &str) {
    bottom_centered_text_with_offset(rc, text, vec2(0., 0.), &[]);
}

pub(crate) fn bottom_center_text(rc: &RenderContext, text: &str, p: Vec2) {
    let p = bottom_center_anchor(rc) + p;
    rc.draw_text(text, p.x, p.y);
}

pub(crate) fn bottom_center_anchor(rc: &RenderContext) -> Vec2 {
    vec2(rc.state.screen_size.x / 2., rc.state.screen_size.y - MARGIN)
}

pub(crate) fn bottom_right_texture(
    rc: &RenderContext,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) -> bool {
    let anchor = vec2(
        rc.state.screen_size.x - MARGIN,
        rc.state.screen_size.y - MARGIN,
    );
    draw_icon(rc, texture, tooltip, anchor + p)
}

pub(crate) fn draw_icon(
    rc: &RenderContext,
    texture: &Texture2D,
    tooltip: &str,
    origin: Vec2,
) -> bool {
    draw_scaled_icon(rc, texture, tooltip, origin, ICON_SIZE)
}

pub(crate) fn draw_scaled_icon(
    rc: &RenderContext,
    texture: &Texture2D,
    tooltip: &str,
    origin: Vec2,
    size: f32,
) -> bool {
    let t = if tooltip.is_empty() {
        vec![]
    } else {
        let mut parts: Vec<String> = vec![];
        break_text(rc, &mut parts, tooltip);
        parts
    };

    draw_scaled_icon_with_tooltip(rc, texture, &t, origin, size)
}

pub(crate) fn draw_scaled_icon_with_tooltip(
    rc: &RenderContext,
    texture: &Texture2D,
    tooltip: &[String],
    origin: Vec2,
    size: f32,
) -> bool {
    if rc.stage.is_main() {
        // todo
        // if rc.stage.is_ui() {
        //     rc.draw_circle(
        //         
        //     )
        // }
        
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
    }

    button_pressed(Rect::new(origin.x, origin.y, size, size), rc, tooltip, 50.)
}

#[must_use]
pub(crate) fn button_pressed(
    rect: Rect,
    rc: &RenderContext,
    tooltip: &[String],
    right_offset: f32,
) -> bool {
    if !tooltip.is_empty() {
        show_tooltip_for_rect(rc, tooltip, rect, right_offset);
    }
    mouse_pressed_position(rc).is_some_and(|p| rect.contains(p))
}

#[must_use]
pub(crate) fn is_in_circle(mouse_pos: Vec2, center: Vec2, radius: f32) -> bool {
    let d = vec2(center.x - mouse_pos.x, center.y - mouse_pos.y);
    d.length() <= radius
}

#[must_use]
pub(crate) fn mouse_pressed_position(rc: &RenderContext) -> Option<Vec2> {
    is_mouse_pressed(rc).then_some(rc.screen_to_world(mouse_position().into()))
}

pub(crate) fn is_mouse_pressed(rc: &RenderContext) -> bool {
    rc.stage.is_tooltip() && is_mouse_button_pressed(MouseButton::Left)
}

pub(crate) fn rect_from(p: Vec2, size: Vec2) -> Rect {
    Rect::new(p.x, p.y, size.x, size.y)
}

pub(crate) fn limit_str(
    s: &str,
    max_width: f32,
    measure: impl Fn(&str) -> TextDimensions,
) -> String {
    let mut limited = String::new();
    for c in s.chars() {
        limited.push(c);
        if measure(&limited).width > max_width {
            limited.pop();
            limited.push_str("..");
            break;
        }
    }
    if limited.is_empty() {
        s.to_string()
    } else {
        limited
    }
}
