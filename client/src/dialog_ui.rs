use macroquad::hash;
use macroquad::math::{vec2, Vec2};
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

use crate::client_state::{ShownPlayer, StateUpdate};

pub fn active_dialog_window<F>(player: &ShownPlayer, title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog(title, |ui| {
            if player.can_control {
                f(ui)
            } else {
                StateUpdate::None
            }
        })
}

pub fn dialog<F>(title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    custom_dialog(title, vec2(1100., 400.), vec2(800., 350.), f)
}

pub fn custom_dialog<F>(title: &str, position: Vec2, size: Vec2, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    let window = Window::new(hash!(), position, size)
        .titlebar(true)
        .movable(false)
        .label(title)
        .close_button(true);

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
