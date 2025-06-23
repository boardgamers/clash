use crate::client_state::StateUpdate;
use crate::dialog_ui::{OkTooltip, cancel_button, ok_button};
use crate::layout_ui::{ICON_SIZE, bottom_center_anchor, bottom_center_texture};
use crate::render_context::RenderContext;
use macroquad::color::{BLACK, BLUE, Color, WHITE};
use macroquad::math::{Vec2, bool, vec2};
use macroquad::prelude::{GRAY, RED};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CountSelector {
    pub current: u8,
    pub min: u8,
    pub max: u8,
}

pub trait HasCountSelectableObject {
    fn counter(&self) -> &CountSelector;
}

#[allow(clippy::too_many_arguments)]
pub fn count_dialog<C, O: HasCountSelectableObject>(
    rc: &RenderContext,
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    draw: impl Fn(&O, Vec2),
    draw_tooltip: impl Fn(&O, Vec2),
    is_valid: impl FnOnce() -> OkTooltip,
    execute_action: impl FnOnce() -> RenderResult,
    show: impl Fn(&C, &O) -> bool,
    plus: impl FnOnce(&C, &O) -> RenderResult,
    minus: impl FnOnce(&C, &O) -> RenderResult,
    offset: Vec2,
    may_cancel: bool,
) -> RenderResult {
    let objects = get_objects(container)
        .into_iter()
        .filter(|o| show(container, o))
        .collect::<Vec<_>>();
    let start_x = objects.len() as f32 * -1. / 2.;
    let anchor = bottom_center_anchor(rc);
    for pass in 0..2 {
        for (i, o) in objects.iter().enumerate() {
            let x = (start_x + i as f32) * ICON_SIZE * 2.;
            let c = o.counter();

            let point = vec2(x + 7., -60.) + anchor + offset;
            if pass == 0 {
                draw(o, point);
                let current_pos = vec2(x + 13., -ICON_SIZE) + anchor + offset;
                rc.state.draw_text_with_color(
                    &format!("{}", c.current),
                    current_pos.x,
                    current_pos.y,
                    BLACK,
                );
                if c.current > c.min
                    && bottom_center_texture(
                        rc,
                        &rc.assets().minus,
                        vec2(x - 15., -ICON_SIZE) + offset,
                        "Remove one",
                    )
                {
                    return minus(container, o);
                }
                if c.current < c.max
                    && bottom_center_texture(
                        rc,
                        &rc.assets().plus,
                        vec2(x + 15., -ICON_SIZE) + offset,
                        "Add one",
                    )
                {
                    return plus(container, o);
                }
            } else {
                draw_tooltip(o, point);
            }
        }
    }

    if ok_button(rc, is_valid()) {
        return execute_action();
    }
    if may_cancel && cancel_button(rc) {
        return StateUpdate::Cancel;
    }

    NO_UPDATE
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HighlightType {
    None,
    Primary,
    Choices,
    Warn,
    Invalid,
}

impl HighlightType {
    pub fn color(self) -> Color {
        match self {
            HighlightType::None => BLACK,
            HighlightType::Primary => WHITE,
            HighlightType::Choices => BLUE,
            HighlightType::Warn => RED,
            HighlightType::Invalid => GRAY,
        }
    }
}
