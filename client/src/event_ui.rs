use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use server::content::action_cards::get_civil_card;
use server::content::advances::get_advance;
use server::content::builtin::get_builtin;
use server::content::incidents::get_incident;
use server::content::objectives::get_objective;
use server::content::tactics_cards::get_tactics_card;
use server::content::wonders::get_wonder;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin) -> Vec<String> {
    let mut h = vec![origin.name()];
    h.extend(match origin {
        EventOrigin::Advance(a) => vec![a.info().description.clone()],
        EventOrigin::Wonder(w) => vec![get_wonder(w).description.clone()],
        EventOrigin::Builtin(b) => vec![get_builtin(rc.game, b).description.clone()],
        EventOrigin::CivilCard(id) => vec![get_civil_card(*id).description.clone()],
        EventOrigin::TacticsCard(id) => vec![get_tactics_card(*id).description.clone()],
        EventOrigin::Incident(id) => get_incident(*id).description(),
        EventOrigin::Objective(name) => vec![get_objective(name).description.clone()],
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
                .find(|sa| &sa.advance == s)
                .unwrap();
            s.description.clone()
        }],
    });
    h
}

#[must_use]
pub fn custom_phase_event_help(rc: &RenderContext, description: &str) -> Vec<String> {
    let mut h = event_help(rc, &custom_phase_event_origin(rc));
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

pub fn pay_help<T>(rc: &RenderContext, p: &Payment<T>) -> Vec<String> {
    let mut result = vec!["Pay resources".to_string()];
    for o in p.cost.modifiers.clone() {
        result.extend(event_help(rc, &o));
    }
    result
}
