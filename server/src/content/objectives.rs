use crate::objective_card::Objective;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<Objective> {
    vec![]
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_objective(name: &str) -> Objective {
    get_all()
        .into_iter()
        .find(|c| c.name == name)
        .expect("objective not found")
}
