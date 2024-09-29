use macroquad::hash;
use macroquad::math::vec2;
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

use crate::client_state::{ShownPlayer, StateUpdate};

pub fn active_dialog_window<F>(player: &ShownPlayer, title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog_window(player, title, false, f)
}

pub fn closeable_dialog_window<F>(title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog(title, true, f)
}

pub fn dialog_window<F>(player: &ShownPlayer, title: &str, close_button: bool, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog(title, close_button, |ui| {
        if player.can_control {
            f(ui)
        } else {
            StateUpdate::None
        }
    })
}

fn dialog<F>(title: &str, close_button: bool, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    let window = Window::new(hash!(), vec2(1100., 400.), vec2(800., 350.))
        .titlebar(true)
        .movable(false)
        .label(title)
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
