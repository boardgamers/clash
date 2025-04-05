mod non_combat;

use crate::content::objectives::non_combat::non_combat_objectives;
use crate::objective_card::Objective;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<Objective> {
    let all = vec![non_combat_objectives()]
        .into_iter()
        .flatten()
        .collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| &i.name).count(),
        all.len(),
        "objective names are not unique"
    );
    all
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
