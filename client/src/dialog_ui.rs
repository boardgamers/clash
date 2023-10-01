use macroquad::hash;
use macroquad::math::vec2;
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

use crate::ui_state::StateUpdate;

pub fn active_dialog_window<F>(f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog_window(false, f)
}

pub fn dialog_window<F>(close_button: bool, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    let window = Window::new(hash!(), vec2(100., 100.), vec2(500., 500.))
        .titlebar(true)
        .movable(true)
        .close_button(close_button);

    let ui = &mut root_ui();
    let token = window.begin(ui);
    let update = f(ui);
    let open = token.end(ui);
    if matches!(update, StateUpdate::None) {
        if open {
            StateUpdate::None
        } else {
            StateUpdate::CloseDialog
        }
    } else {
        update
    }
}
