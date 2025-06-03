use crate::payment_ui::Payment;
use crate::render_context::RenderContext;
use crate::tooltip::add_tooltip_description;
use server::events::EventOrigin;

#[must_use]
pub fn event_help(rc: &RenderContext, origin: &EventOrigin) -> Vec<String> {
    let mut h = vec![origin.name(rc.game)];
    let cache = &rc.game.cache;
    let d = match origin {
        EventOrigin::Advance(a) => vec![a.info(rc.game).description.clone()],
        EventOrigin::Wonder(w) => vec![rc.game.cache.get_wonder(*w).description.clone()],
        EventOrigin::Ability(b) => vec![
            cache
                .get_ability_description(b, rc.game, rc.shown_player)
                .clone(),
        ],
        EventOrigin::CivilCard(id) => vec![cache.get_civil_card(*id).description.clone()],
        EventOrigin::TacticsCard(id) => vec![cache.get_tactics_card(*id).description.clone()],
        EventOrigin::Incident(id) => cache.get_incident(*id).description(rc.game),
        EventOrigin::Objective(name) => vec![cache.get_objective(name).description.clone()],
        EventOrigin::LeaderAbility(l) => {
            vec![rc.shown_player.get_leader_ability(l).description.clone()]
        }
        EventOrigin::SpecialAdvance(s) => vec![s.info(rc.game).description.clone()],
    };
    h.extend(d);
    h
}

#[must_use]
pub fn custom_phase_event_help(rc: &RenderContext, description: &str) -> Vec<String> {
    let mut h = event_help(rc, &custom_phase_event_origin(rc));
    add_tooltip_description(&mut h, description);
    h
}

#[must_use]
pub fn custom_phase_event_origin(rc: &RenderContext) -> EventOrigin {
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

pub fn pay_help<T: Clone>(rc: &RenderContext, p: &Payment<T>) -> Vec<String> {
    let mut result = vec!["Pay resources".to_string()];
    for o in p.cost.modifiers.clone() {
        result.extend(event_help(rc, &o));
    }
    result
}
