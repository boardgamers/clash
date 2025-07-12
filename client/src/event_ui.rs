use crate::log_ui::MultilineText;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::effects;
use server::events::EventOrigin;

#[must_use]
pub(crate) fn event_help_tooltip(rc: &RenderContext, origin: &EventOrigin) -> MultilineText {
    let mut help = MultilineText::default();
    let labels = &event_help(rc, origin);
    for label in labels {
        help.add(rc, label);
    }
    help
}

#[must_use]
pub(crate) fn event_help(rc: &RenderContext, origin: &EventOrigin) -> Vec<String> {
    effects::event_help(rc.game, origin)
}

#[must_use]
pub(crate) fn custom_phase_event_help(rc: &RenderContext, description: &str) -> Vec<String> {
    let mut h = event_help(rc, &custom_phase_event_origin(rc));
    h.push(description.to_string());
    h
}

#[must_use]
pub(crate) fn custom_phase_event_origin(rc: &RenderContext) -> EventOrigin {
    rc.game
        .current_event()
        .origin_override
        .clone()
        .unwrap_or_else(|| {
            rc.game
                .current_event_handler()
                .as_ref()
                .unwrap()
                .origin
                .clone()
        })
}

pub(crate) fn pay_help<T: Clone>(rc: &RenderContext, p: &Payment<T>) -> Vec<String> {
    let mut result = vec!["Pay resources".to_string()];
    for o in p.cost.modifiers.clone() {
        result.extend(event_help(rc, &o));
    }
    result
}
