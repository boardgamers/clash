use crate::log_ui::break_text;
use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::action_cards::get_civil_card;
use server::content::advances::get_advance;
use server::content::builtin::get_builtin;
use server::content::incidents::get_incident;
use server::content::tactics_cards::get_tactics_card;
use server::content::wonders::get_wonder;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin, do_break: bool) -> Vec<String> {
    let mut h = vec![origin.name()];
    h.extend(match origin {
        EventOrigin::Advance(a) => vec![get_advance(a).description],
        EventOrigin::Wonder(w) => vec![get_wonder(w).description],
        EventOrigin::Builtin(b) => vec![get_builtin(rc.game, b).description],
        EventOrigin::CivilCard(id) => vec![get_civil_card(*id).description],
        EventOrigin::TacticsCard(name) => vec![get_tactics_card(name).description],
        EventOrigin::Incident(id) => get_incident(*id).description(),
        EventOrigin::Leader(l) => vec![{
            let l = rc.shown_player.get_leader(l).unwrap();
            // todo: leader should have a 2 event sources - no each event source should have a description
            format!(
                "{}, {}",
                l.first_ability_description, l.second_ability_description
            )
        }],
        EventOrigin::SpecialAdvance(s) => vec![{
            let s = rc
                .shown_player
                .civilization
                .special_advances
                .iter()
                .find(|sa| &sa.name == s)
                .unwrap();
            s.description.clone()
        }],
    });
    if do_break {
        h.iter()
            .flat_map(|s| {
                let mut result = vec![];
                break_text(s, 30, &mut result);
                result
            })
            .collect()
    } else {
        h
    }
}

#[must_use]
pub fn custom_phase_event_help(rc: &RenderContext, description: &str) -> Vec<String> {
    let mut h = event_help(rc, &custom_phase_event_origin(rc), true);
    h.push(description.to_string());
    h
}

#[must_use]
pub fn custom_phase_event_origin(rc: &RenderContext) -> EventOrigin {
    rc.game
        .current_event_handler()
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
