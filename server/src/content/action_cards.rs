mod inspiration;

use crate::action_card::ActionCard;
use crate::content::action_cards::inspiration::inspiration_action_cards;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> Vec<ActionCard> {
    let all = vec![inspiration_action_cards()]
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
        .expect("action card not found")
}
