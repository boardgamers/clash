use crate::client_state::{PendingUpdate, StateUpdate};
use crate::layout_ui::{bottom_centered_text_with_offset, bottom_right_texture, icon_pos};
use crate::render_context::RenderContext;
use macroquad::math::{Vec2, vec2};
use server::playing_actions::PlayingActionType;

#[derive(Clone)]
pub enum OkTooltip {
    Valid(String),
    Invalid(String),
}

#[derive(Clone)]
pub struct BaseOrCustomDialog {
    pub title: String,
    pub action_type: PlayingActionType,
}

pub fn show_pending_update(update: &PendingUpdate, rc: &RenderContext) -> StateUpdate {
    let state = &rc.state;
    let t = if update.warning.is_empty() {
        if state.active_dialog.is_modal() {
            &update.info.join(", ")
        } else {
            "Are you sure?"
        }
    } else {
        &format!("Warning: {}", update.warning.join(", "))
    };
    bottom_centered_text_with_offset(rc, t, vec2(0., 30.), &[]);

    if ok_button(rc, OkTooltip::Valid("OK".to_string())) {
        return StateUpdate::ResolvePendingUpdate(true);
    }
    if cancel_button(rc) {
        return StateUpdate::ResolvePendingUpdate(false);
    }
    StateUpdate::None
}

#[must_use]
pub fn cancel_button(rc: &RenderContext) -> bool {
    cancel_button_with_tooltip(rc, "Cancel")
}

#[must_use]
pub fn cancel_button_with_tooltip(rc: &RenderContext, tooltip: &str) -> bool {
    bottom_right_texture(rc, &rc.assets().cancel, cancel_button_pos(), tooltip)
}

#[must_use]
pub fn cancel_button_pos() -> Vec2 {
    icon_pos(-7, -1)
}

#[must_use]
pub fn ok_button(rc: &RenderContext, ok_tooltip: OkTooltip) -> bool {
    let pos = icon_pos(-8, -1);
    match ok_tooltip {
        OkTooltip::Valid(tooltip) => bottom_right_texture(rc, &rc.assets().ok, pos, &tooltip),
        OkTooltip::Invalid(tooltip) => {
            let _ = bottom_right_texture(rc, &rc.assets().ok_blocked, pos, &tooltip);
            false
        }
    }
}
