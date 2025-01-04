use crate::client_state::{PendingUpdate, ShownPlayer, State, StateUpdate};
use crate::layout_ui::{bottom_center_text, bottom_right_texture, icon_pos};
use macroquad::hash;
use macroquad::math::{vec2, Vec2};
use macroquad::ui::widgets::Window;
use macroquad::ui::{root_ui, Ui};

pub fn dialog<F>(player: &ShownPlayer, title: &str, f: F) -> StateUpdate
where
    F: FnOnce(&mut Ui) -> StateUpdate,
{
    let size = player.screen_size;
    let width = size.x - 20.;
    let size = vec2(width, size.y - 40.);
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

pub fn show_pending_update(update: &PendingUpdate, state: &State) -> StateUpdate {
    let t = if update.warning.is_empty() {
        "Are you sure?"
    } else {
        &format!("Warning: {}", update.warning.join(", "))
    };
    let dimensions = state.measure_text(t);
    bottom_center_text(state, t, vec2(-dimensions.width / 2., -50.));

    if ok_button(state, update.can_confirm) {
        return StateUpdate::ResolvePendingUpdate(true);
    }
    if cancel_button(state) {
        return StateUpdate::ResolvePendingUpdate(false);
    }
    StateUpdate::None
}

#[must_use]
pub fn cancel_button(state: &State) -> bool {
    cancel_button_with_tooltip(state, "Cancel")
}

#[must_use]
pub fn cancel_button_with_tooltip(state: &State, tooltip: &str) -> bool {
    bottom_right_texture(state, &state.assets.cancel, icon_pos(-7, -1), tooltip)
}

#[must_use]
pub fn ok_button(state: &State, valid: bool) -> bool {
    ok_button_with_tooltip(state, valid, if valid { "OK" } else { "Invalid selection" })
}

#[must_use]
pub fn ok_button_with_tooltip(state: &State, valid: bool, tooltip: &str) -> bool {
    let ok = if valid {
        &state.assets.ok
    } else {
        &state.assets.ok_blocked
    };
    bottom_right_texture(state, ok, icon_pos(-8, -1), tooltip) && valid
}
