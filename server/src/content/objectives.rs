pub(crate) mod combat;
pub(crate) mod non_combat;
pub(crate) mod status_phase_objectives;

use crate::objective_card::Objective;
use combat::combat_objectives;
use itertools::Itertools;
use non_combat::non_combat_objectives;
use status_phase_objectives::status_phase_objectives;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<Objective> {
    let all = vec![
        combat_objectives(),
        non_combat_objectives(),
        status_phase_objectives(),
    ]
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
