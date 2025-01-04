use crate::client_state::{PendingUpdate, State, StateUpdate};
use crate::layout_ui::{bottom_center_text, bottom_right_texture, icon_pos};
use macroquad::math::vec2;

pub fn show_pending_update(update: &PendingUpdate, state: &State) -> StateUpdate {
    let t = if update.warning.is_empty() {
        if state.active_dialog.is_full_modal() {
            &update.info.join(", ")
        } else {
            "Are you sure?"
        }
    } else {
        &format!("Warning: {}", update.warning.join(", "))
    };
    let dimensions = state.measure_text(t);
    bottom_center_text(state, t, vec2(-dimensions.width / 2., -50.));

    if ok_button(state, true) {
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
