use macroquad::hash;
use macroquad::math::{vec2, Vec2};
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

use crate::client_state::{PendingUpdate, ShownPlayer, StateUpdate};

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

pub fn full_dialog<F>(title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    custom_dialog(title, vec2(100., 100.), vec2(1600., 800.), f)
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

    let (update, open) = show_window(window, f);
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

pub fn show_window<F, R>(window: Window, f: F) -> (R, bool)
where
    F: FnOnce(&mut Ui) -> R,
{
    let ui = &mut root_ui();
    let token = window.begin(ui);
    let update = f(ui);
    let open = token.end(ui);
    (update, open)
}

pub fn show_pending_update(update: &PendingUpdate, player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(player, "Are you sure?", |ui| {
        ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
        if ui.button(None, "OK") {
            return StateUpdate::ResolvePendingUpdate(true);
        }
        if ui.button(None, "Cancel") {
            return StateUpdate::ResolvePendingUpdate(false);
        }
        StateUpdate::None
    })
}
