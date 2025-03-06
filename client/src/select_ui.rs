use crate::client_state::StateUpdate;
use crate::dialog_ui::{cancel_button, cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::layout_ui::{bottom_center_anchor, bottom_center_texture, ICON_SIZE};
use crate::render_context::RenderContext;
use macroquad::color::{Color, BLACK, BLUE, WHITE};
use macroquad::math::{bool, vec2, Vec2};
use macroquad::prelude::TextParams;
use macroquad::text::draw_text_ex;
use server::game::Game;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CountSelector {
    pub current: u32,
    pub min: u32,
    pub max: u32,
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
    is_valid: impl FnOnce() -> OkTooltip,
    execute_action: impl FnOnce() -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl FnOnce(&C, &O) -> StateUpdate,
    minus: impl FnOnce(&C, &O) -> StateUpdate,
    offset: Vec2,
    may_cancel: bool,
) -> StateUpdate {
    let objects = get_objects(container)
        .into_iter()
        .filter(|o| show(container, o))
        .collect::<Vec<_>>();
    let start_x = objects.len() as f32 * -1. / 2.;
    let anchor = bottom_center_anchor(rc);
    for (i, o) in objects.iter().enumerate() {
        let x = (start_x + i as f32) * ICON_SIZE * 2.;
        let c = o.counter();

        draw(o, vec2(x + 7., -60.) + anchor + offset);
        let current_pos = vec2(x + 13., -ICON_SIZE) + anchor + offset;
        draw_text_ex(
            &format!("{}", c.current),
            current_pos.x,
            current_pos.y,
            TextParams {
                font_size: 20,
                font_scale: 1.,
                font: Some(&rc.assets().font),
                color: BLACK,
                ..Default::default()
            },
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
        };
    }

    if ok_button(rc, is_valid()) {
        return execute_action();
    }
    if may_cancel && cancel_button(rc) {
        return StateUpdate::Cancel;
    };

    StateUpdate::None
}

pub trait ConfirmSelection: Clone {
    fn cancel_name(&self) -> Option<&str> {
        Some("Cancel")
    }

    fn confirm(&self, game: &Game) -> OkTooltip;
}

pub fn may_cancel(sel: &impl ConfirmSelection, rc: &RenderContext) -> StateUpdate {
    if let Some(cancel_name) = sel.cancel_name() {
        if cancel_button_with_tooltip(rc, cancel_name) {
            StateUpdate::Cancel
        } else {
            StateUpdate::None
        }
    } else {
        StateUpdate::None
    }
}

#[derive(Clone, Copy)]
pub enum HighlightType {
    None,
    Primary,
    Choices,
}

impl HighlightType {
    pub fn color(self) -> Color {
        match self {
            HighlightType::None => BLACK,
            HighlightType::Primary => WHITE,
            HighlightType::Choices => BLUE,
        }
    }
}
