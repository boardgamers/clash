use crate::content::objectives::non_combat::draft;
use crate::objective_card::{Objective, ObjectiveCard};
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<ObjectiveCard> {
    vec![
        ObjectiveCard::new(
            1,
            Objective::builder("Objective 1", "Description 1").build(), //todo
            draft(),
        ),
        // todo replace when we have a real repeated objective
        ObjectiveCard::new(
            99,
            Objective::builder("Objective 1", "Description 1").build(), //todo
            draft(),
        ),
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
