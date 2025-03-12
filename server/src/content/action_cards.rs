use crate::action_card::ActionCard;
use crate::content::tactics_cards::peltasts;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> Vec<ActionCard> {
    let all = vec![test_action_cards()]
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

fn test_action_cards() -> Vec<ActionCard> {
    vec![advance(1), advance(2)]
}

// todo move to dedicated module
fn advance(id: u8) -> ActionCard {
    ActionCard::civil_card_builder(id, "Advance", "todo", peltasts())
        // .add_units_request()
        .build()
}
