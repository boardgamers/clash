use crate::log_ui::break_text;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::advances::get_advance;
use server::content::wonders::get_wonder;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin, do_break: bool) -> Vec<String> {
    let h = match origin {
        EventOrigin::Advance(a) => get_advance(a).description,
        EventOrigin::Wonder(w) => get_wonder(w).description,
        EventOrigin::Leader(l) => {
            let l = rc.shown_player.get_leader(l).unwrap();
            // todo: leader should have a 2 event sources
            format!("{}: {}, {}", l.name, l.first_ability_description, l.second_ability_description)
        }
        EventOrigin::SpecialAdvance(s) => {
            let s = rc.shown_player.civilization.special_advances.iter().find(|sa| &sa.name == s).unwrap();
            s.description.clone()
        },
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
