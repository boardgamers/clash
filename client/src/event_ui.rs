use crate::log_ui::break_text;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::advances::get_advance;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(origin: &EventOrigin, do_break: bool) -> Vec<String> {
    let h = match origin {
        EventOrigin::Advance(a) => get_advance(a).description,
        _ => String::new(), // TODO
    };
    if do_break {
        let mut result = vec![];
        break_text(&format!("{origin:?}"), 30, &mut result);
        result
    } else {
        vec![h]
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

pub fn pay_help(p: &Payment) -> Vec<String> {
    let mut result = vec!["Pay resources".to_string()];
    for o in p.cost.modifiers.clone() {
        result.extend(event_help(&o, true));
    }
    result
}
