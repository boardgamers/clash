use crate::client_state::{ShownPlayer, State, StateUpdate, StateUpdates};
use crate::dialog_ui::active_dialog_window;
use crate::layout_ui::{bottom_center_anchor, bottom_center_texture, bottom_right_texture, cancel_pos, icon_pos, ok_pos, ICON_SIZE};
use macroquad::color::BLACK;
use macroquad::hash;
use macroquad::math::{bool, vec2, Vec2};
use macroquad::prelude::TextParams;
use macroquad::text::draw_text_ex;
use macroquad::ui::widgets::Group;
use macroquad::ui::Ui;
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

// old code - use count_dialog_icon instead
#[allow(clippy::too_many_arguments)]
pub fn count_dialog<C, O: HasCountSelectableObject>(
    player: &ShownPlayer,
    title: &str,
    info: Vec<String>,
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    label: impl Fn(&O) -> &str,
    is_valid: impl FnOnce(&C) -> bool,
    execute_action: impl FnOnce(&C) -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl Fn(&C, &O) -> StateUpdate,
    minus: impl Fn(&C, &O) -> StateUpdate,
) -> StateUpdate {
    active_dialog_window(player, title, |ui| {
        for i in info {
            ui.label(None, &i);
        }
        let mut updates = StateUpdates::new();
        for (i, p) in get_objects(container).iter().enumerate() {
            if show(container, p) {
                Group::new(hash!("res", i), Vec2::new(120., 150.)).ui(ui, |ui| {
                    let c = p.counter();
                    ui.label(Vec2::new(0., 0.), &format!("{} {}", &label(p), c.current));
                    if c.current > c.min && ui.button(Vec2::new(0., 80.), "-") {
                        updates.add(minus(container, p));
                    }
                    if c.current < c.max && ui.button(Vec2::new(0., 40.), "+") {
                        updates.add(plus(container, p));
                    };
                });
            }
        }

        let valid = is_valid(container);
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(ok_pos(player), label) && valid {
            return execute_action(container);
        };
        if ui.button(cancel_pos(player), "Cancel") {
            return StateUpdate::Cancel;
        };

        updates.result()
    })
}

#[allow(clippy::too_many_arguments)]
pub fn count_dialog_icon<C, O: HasCountSelectableObject>(
    player: &ShownPlayer,
    state: &State,
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    draw: impl Fn(&O, Vec2),
    is_valid: impl FnOnce(&C) -> bool,
    execute_action: impl FnOnce(&C) -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl Fn(&C, &O) -> StateUpdate,
    minus: impl Fn(&C, &O) -> StateUpdate,
) -> StateUpdate {
    if !player.can_control {
        return StateUpdate::None;
    }

    let mut updates = StateUpdates::new();
    let objects = get_objects(container).into_iter().filter(|o| show(container, o)).collect::<Vec<_>>();
    let start_x = objects.len() as f32 * -1. / 2.;
    let anchor = bottom_center_anchor(state);
    for (i, o) in objects.iter().enumerate() {
        let x = (start_x + i as f32) * ICON_SIZE * 2.;
        let c = o.counter();

        draw(o, vec2(x + 15., -60.) + anchor);
        draw_text_ex(
            &format!("{}", c.current),
            anchor.x + x + 15.,
            anchor.y -ICON_SIZE,
            TextParams {
                font_size: 20,
                font_scale: 1.,
                font: Some(&state.assets.font),
                color: BLACK,
                ..Default::default()
            },
        );
        if c.current > c.min && bottom_center_texture(state, &state.assets.minus, vec2(x - 15., -ICON_SIZE), "Remove one") {
            updates.add(minus(container, o));
        }
        if c.current < c.max && bottom_center_texture(state, &state.assets.plus, vec2(x + 15., -ICON_SIZE), "Add one") {
            updates.add(plus(container, o));
        };
    }

    let valid = is_valid(container);
    let ok = if valid { &state.assets.ok } else { &state.assets.ok_blocked };
    let ok_tooltip = if valid { "OK" } else { "Invalid selection" };
    if bottom_right_texture(state, ok, icon_pos(-8,-1), ok_tooltip) && valid {
        return execute_action(container);
    };
    if bottom_right_texture(state, &state.assets.cancel, icon_pos(-7,-1), "Cancel") {
        return StateUpdate::Cancel;
    };

    updates.result()
}

pub trait ConfirmSelection: Clone {
    fn cancel_name(&self) -> Option<&str> {
        Some("Cancel")
    }

    fn cancel(&self) -> StateUpdate {
        StateUpdate::Cancel
    }

    fn confirm(&self, game: &Game) -> SelectionConfirm;
}

pub trait Selection: ConfirmSelection {
    fn all(&self) -> &[String];
    fn selected(&self) -> &[String];
    fn selected_mut(&mut self) -> &mut Vec<String>;
    fn can_select(&self, game: &Game, name: &str) -> bool;
}

pub fn selection_dialog<T: Selection>(
    game: &Game,
    player: &ShownPlayer,
    title: &str,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
    on_ok: impl FnOnce(T) -> StateUpdate,
) -> StateUpdate {
    active_dialog_window(player, title, |ui| {
        for name in sel.all() {
            let can_sel = sel.can_select(game, name);
            let is_selected = sel.selected().contains(name);
            let mut l = name.to_string();
            if is_selected {
                l += " (selected)";
            }

            if !can_sel {
                ui.label(None, &l);
            } else if ui.button(None, l) {
                let mut new = sel.clone();
                if is_selected {
                    new.selected_mut().retain(|n| n != name);
                } else {
                    new.selected_mut().push(name.to_string());
                }
                return on_change(new);
            }
        }
        confirm_update(sel, player, || on_ok(sel.clone()), ui, &sel.confirm(game))
    })
}

pub fn confirm_update<T: ConfirmSelection>(
    sel: &T,
    player: &ShownPlayer,
    on_ok: impl FnOnce() -> StateUpdate,
    ui: &mut Ui,
    confirm: &SelectionConfirm,
) -> StateUpdate {
    match confirm {
        SelectionConfirm::NoConfirm => StateUpdate::None,
        SelectionConfirm::Invalid => {
            ui.label(ok_pos(player), "Invalid selection");
            may_cancel(sel, ui)
        }
        SelectionConfirm::Valid => {
            if ui.button(ok_pos(player), "OK") {
                on_ok()
            } else {
                may_cancel(sel, ui)
            }
        }
    }
}

fn may_cancel(sel: &impl ConfirmSelection, ui: &mut Ui) -> StateUpdate {
    if let Some(cancel_name) = sel.cancel_name() {
        if ui.button(None, cancel_name) {
            sel.cancel()
        } else {
            StateUpdate::None
        }
    } else {
        StateUpdate::None
    }
}

pub enum SelectionConfirm {
    NoConfirm,
    Invalid,
    Valid,
}
