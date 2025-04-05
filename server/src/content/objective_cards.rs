use crate::content::objectives::non_combat::draft;
use crate::content::objectives::status_phase_objectives::large_civ;
use crate::objective_card::ObjectiveCard;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<ObjectiveCard> {
    vec![
        ObjectiveCard::new(1, large_civ(), draft()),
        // todo replace when we have a real repeated objective
        ObjectiveCard::new(99, large_civ(), draft()),
    ]
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_objective_card(id: u8) -> ObjectiveCard {
    get_all()
        .into_iter()
        .find(|c| c.id == id)
        .expect("objective card not found")
}
