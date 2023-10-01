use crate::ui_state::StateUpdate;
use macroquad::hash;
use macroquad::math::vec2;
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

pub fn active_dialog_window<F>(f: F)
where
    F: FnOnce(&mut Ui),
{
    let _ = dialog_window(false, f);
}

pub fn dialog_window<F>(close_button: bool, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui),
{
    let open = Window::new(hash!(), vec2(100., 100.), vec2(500., 500.))
        .titlebar(true)
        .movable(true)
        .close_button(close_button)
        .ui(&mut root_ui(), f);
    if open {
        StateUpdate::None
    } else {
        StateUpdate::CloseDialog
    }
}
