use crate::client_state::{State, StateUpdate, StateUpdates};
use crate::dialog_ui::{cancel_button, ok_button, OkTooltip};
use crate::layout_ui::{bottom_center_anchor, bottom_center_texture, ICON_SIZE};
use macroquad::color::BLACK;
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
    fn counter_mut(&mut self) -> &mut CountSelector;
}

#[allow(clippy::too_many_arguments)]
pub fn count_dialog<C, O: HasCountSelectableObject>(
    state: &State,
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    draw: impl Fn(&O, Vec2),
    is_valid: impl FnOnce(&C) -> OkTooltip,
    execute_action: impl FnOnce(&C) -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl Fn(&C, &O) -> StateUpdate,
    minus: impl Fn(&C, &O) -> StateUpdate,
) -> StateUpdate {
    let mut updates = StateUpdates::new();
    let objects = get_objects(container)
        .into_iter()
        .filter(|o| show(container, o))
        .collect::<Vec<_>>();
    let start_x = objects.len() as f32 * -1. / 2.;
    let anchor = bottom_center_anchor(state);
    for (i, o) in objects.iter().enumerate() {
        let x = (start_x + i as f32) * ICON_SIZE * 2.;
        let c = o.counter();

        draw(o, vec2(x + 15., -60.) + anchor);
        draw_text_ex(
            &format!("{}", c.current),
            anchor.x + x + 15.,
            anchor.y - ICON_SIZE,
            TextParams {
                font_size: 20,
                font_scale: 1.,
                font: Some(&state.assets.font),
                color: BLACK,
                ..Default::default()
            },
        );
        if c.current > c.min
            && bottom_center_texture(
                state,
                &state.assets.minus,
                vec2(x - 15., -ICON_SIZE),
                "Remove one",
            )
        {
            updates.add(minus(container, o));
        }
        if c.current < c.max
            && bottom_center_texture(
                state,
                &state.assets.plus,
                vec2(x + 15., -ICON_SIZE),
                "Add one",
            )
        {
            updates.add(plus(container, o));
        };
    }

    if ok_button(state, is_valid(container)) {
        return execute_action(container);
    }
    if cancel_button(state) {
        return StateUpdate::Cancel;
    };

    updates.result()
}

pub trait ConfirmSelection: Clone {
    fn cancel_name(&self) -> Option<&str> {
        Some("Cancel")
    }

    fn confirm(&self, game: &Game) -> OkTooltip;
}
