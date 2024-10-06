use crate::client_state::{PendingUpdate, ShownPlayer, StateUpdate};
use crate::layout_ui::{cancel_pos, ok_pos};
use macroquad::hash;
use macroquad::math::{vec2, Vec2};
use macroquad::prelude::screen_height;
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};
use macroquad::window::screen_width;

pub fn active_dialog_window<F>(player: &ShownPlayer, title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    dialog(player, title, |ui| {
        if player.can_control {
            f(ui)
        } else {
            StateUpdate::None
        }
    })
}

pub fn dialog<F>(player: &ShownPlayer, title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    let width = screen_width() - 20.;
    let size = if player.active_dialog.is_map_dialog() {
        vec2(width / 2.0, 270.)
    } else {
        vec2(width, screen_height() - 40.)
    };
    custom_dialog(title, vec2(10., 10.), size, f)
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

fn show_window<F, R>(window: Window, f: F) -> (R, bool)
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
        if let Some(ref message) = update.info {
            ui.label(None, message);
        }
        if !update.warning.is_empty() {
            ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
        }
        if ui.button(ok_pos(player), "OK") {
            return StateUpdate::ResolvePendingUpdate(true);
        }
        if ui.button(cancel_pos(player), "Cancel") {
            return StateUpdate::ResolvePendingUpdate(false);
        }
        StateUpdate::None
    })
}
