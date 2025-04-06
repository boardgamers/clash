pub(crate) mod combat_objectives;
pub(crate) mod non_combat;
pub(crate) mod status_phase_objectives;

use crate::content::objective_cards;
use crate::objective_card::Objective;
use itertools::Itertools;

#[must_use]
pub(crate) fn get_all() -> Vec<Objective> {
    let mut all = objective_cards::get_all()
        .into_iter()
        .flat_map(|card| card.objectives.map(|o| (o.name.clone(), o)))
        .collect_vec();
    all.sort_by_key(|(name, _)| name.clone());
    all.dedup_by_key(|(name, _)| name.clone());
    let all = all.into_iter().map(|(_, o)| o).collect_vec();
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
