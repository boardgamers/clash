use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{StateUpdate, StateUpdates};
use macroquad::hash;
use macroquad::math::{bool, Vec2};
use macroquad::ui::widgets::Group;

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
    container: &C,
    get_objects: impl Fn(&C) -> Vec<O>,
    label: impl Fn(&O) -> &str,
    is_valid: impl FnOnce(&C) -> bool,
    execute_action: impl FnOnce(&C) -> StateUpdate,
    show: impl Fn(&C, &O) -> bool,
    plus: impl Fn(&C, &O) -> StateUpdate,
    minus: impl Fn(&C, &O) -> StateUpdate,
) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        for (i, p) in get_objects(container).iter().enumerate() {
            if show(container, p) {
                Group::new(hash!("res", i), Vec2::new(100., 40.)).ui(ui, |ui| {
                    let c = p.counter();
                    ui.label(Vec2::new(0., 0.), &format!("{} {}", &label(p), c.current));
                    if c.current > c.min && ui.button(Vec2::new(0., 20.), "-") {
                        updates.add(minus(container, p));
                    }
                    if c.current < c.max && ui.button(Vec2::new(20., 20.), "+") {
                        updates.add(plus(container, p));
                    };
                });
            }
        }

        let valid = is_valid(container);
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(20., 160.), label) && valid {
            updates.add(execute_action(container));
        };
        if ui.button(Vec2::new(80., 160.), "Cancel") {
            updates.add(StateUpdate::Cancel);
        };
    });
    updates.result()
}
