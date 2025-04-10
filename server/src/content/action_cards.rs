pub(crate) mod cultural_takeover;
pub(crate) mod development;
mod inspiration;
mod mercenaries;
pub(crate) mod negotiation;
pub(crate) mod spy;
pub(crate) mod synergies;

use crate::action_card::{ActionCard, CivilCard};
use crate::content::action_cards::inspiration::inspiration_action_cards;
use crate::content::action_cards::negotiation::negotiation_action_cards;
use crate::content::action_cards::synergies::synergies_action_cards;
use crate::content::incidents;
use development::development_action_cards;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> Vec<ActionCard> {
    let all = vec![
        inspiration_action_cards(),
        development_action_cards(),
        negotiation_action_cards(),
        synergies_action_cards(),
    ]
    .into_iter()
    .flatten()
    .collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| i.id).count(),
        all.len(),
        "action card ids are not unique"
    );
    all
}

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_action_card(id: u8) -> ActionCard {
    get_all()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| {
            incidents::get_all()
                .into_iter()
                .find_map(|incident| incident.action_card.filter(|a| a.id == id))
                .expect("incident action card not found")
        })
}

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_civil_card(id: u8) -> CivilCard {
    get_action_card(id).civil_card
}
