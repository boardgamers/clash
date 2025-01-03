use crate::client_state::{PendingUpdate, ShownPlayer, State, StateUpdate};
use crate::layout_ui::{bottom_right_texture, cancel_pos, icon_pos, ok_only_pos, ok_pos};
use macroquad::hash;
use macroquad::math::{vec2, Vec2};
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

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
    let size = player.screen_size;
    let width = size.x - 20.;
    let size =
        vec2(width, size.y - 40.);
    custom_dialog(title, vec2(10., 10.), size, f)
}

fn custom_dialog<F>(title: &str, position: Vec2, size: Vec2, f: F) -> StateUpdate
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
        for i in &update.info {
            ui.label(None, i);
        }
        if !update.warning.is_empty() {
            ui.label(None, &format!("Warning: {}", update.warning.join(", ")));
        }
        if update.can_confirm && ui.button(ok_pos(player), "OK") {
            return StateUpdate::ResolvePendingUpdate(true);
        }
        let p = if update.can_confirm {
            cancel_pos(player)
        } else {
            ok_only_pos(player)
        };
        if ui.button(p, "Cancel") {
            return StateUpdate::ResolvePendingUpdate(false);
        }
        StateUpdate::None
    })
}

pub fn cancel_button(state: &State) -> bool {
    bottom_right_texture(state, &state.assets.cancel, icon_pos(-7, -1), "Cancel")
}

pub fn ok_button(state: &State, valid: bool) -> bool {
    let ok = if valid {
        &state.assets.ok
    } else {
        &state.assets.ok_blocked
    };
    let ok_tooltip = if valid { "OK" } else { "Invalid selection" };
    bottom_right_texture(state, ok, icon_pos(-8, -1), ok_tooltip) && valid
}
