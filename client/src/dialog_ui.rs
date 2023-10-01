use crate::ui_state::{StateUpdate, StateUpdates};
use macroquad::hash;
use macroquad::math::vec2;
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

pub fn active_dialog_window<F>(f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui, &mut StateUpdates),
{
    dialog_window(false, f)
}

pub fn dialog_window<F>(close_button: bool, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui, &mut StateUpdates),
{
    let mut updates = StateUpdates::new();
    let window = Window::new(hash!(), vec2(100., 100.), vec2(500., 500.))
        .titlebar(true)
        .movable(true)
        .close_button(close_button);

    let ui = &mut root_ui();
    let token = window.begin(ui);
    f(ui, &mut updates);
    let open = token.end(ui);
    let update = if open {
        StateUpdate::None
    } else {
        StateUpdate::CloseDialog
    };
    updates.add(update);
    updates.result()
}
