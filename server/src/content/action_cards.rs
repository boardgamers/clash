pub(crate) mod cultural_takeover;
pub(crate) mod development;
mod inspiration;
mod mercenaries;
pub(crate) mod negotiation;
pub(crate) mod spy;
pub(crate) mod synergies;

use crate::action_card::{ActionCard, CivilCard};
use crate::cache;
use crate::content::action_cards::inspiration::inspiration_action_cards;
use crate::content::action_cards::negotiation::negotiation_action_cards;
use crate::content::action_cards::synergies::synergies_action_cards;
use development::development_action_cards;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> &'static Vec<ActionCard> {
    cache::get().get_action_cards()
}

#[must_use]
pub(crate) fn get_all_uncached() -> Vec<ActionCard> {
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
pub fn get_action_card(id: u8) -> &'static ActionCard {
    cache::get()
        .get_action_card(id)
        .expect("incident action card not found")
}

///
/// # Panics
/// Panics if action card does not exist
#[must_use]
pub fn get_civil_card(id: u8) -> &'static CivilCard {
    &get_action_card(id).civil_card
}
