use crate::log_ui::break_text;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::advances::get_advance;
use server::content::builtin::get_builtin;
use server::content::incidents::get_incident;
use server::content::wonders::get_wonder;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin, do_break: bool) -> Vec<String> {
    let h = match origin {
        EventOrigin::Advance(a) => get_advance(a).description,
        EventOrigin::Wonder(w) => get_wonder(w).description,
        EventOrigin::Builtin(b) => get_builtin(b).description,
        EventOrigin::Incident(id) => get_incident(*id).description(),
        EventOrigin::Leader(l) => {
            let l = rc.shown_player.get_leader(l).unwrap();
            // todo: leader should have a 2 event sources
            format!(
                "{}, {}",
                l.first_ability_description, l.second_ability_description
            )
        }
        EventOrigin::SpecialAdvance(s) => {
            let s = rc
                .shown_player
                .civilization
                .special_advances
                .iter()
                .find(|sa| &sa.name == s)
                .unwrap();
            s.description.clone()
        }
    };
    let h = format!("{}: {}", origin.name(), h);
    if do_break {
        let mut result = vec![];
        break_text(&h, 30, &mut result);
        result
    } else {
        vec![h]
    }
}

#[must_use]
pub fn custom_phase_event_help(rc: &RenderContext, description: Option<&String>) -> Vec<String> {
    let mut h = event_help(rc, &custom_phase_event_origin(rc), true);
    if let Some(d) = description {
        h.push(d.clone());
    }
    h
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

pub fn pay_help(rc: &RenderContext, p: &Payment) -> Vec<String> {
    let mut result = vec!["Pay resources".to_string()];
    for o in p.cost.modifiers.clone() {
        result.extend(event_help(rc, &o, true));
    }
    result
}
