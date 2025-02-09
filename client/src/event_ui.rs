use crate::log_ui::advance_help;
use crate::render_context::RenderContext;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin) -> Vec<String> {
    match origin {
        EventOrigin::Advance(a) => advance_help(rc, a),
        _ => vec![], // TODO
    }
}

#[must_use]
pub fn custom_phase_event_origin(rc: &RenderContext) -> EventOrigin {
    rc.game
        .custom_phase_state
        .current
        .as_ref()
        .unwrap()
        .origin
        .clone()
}
