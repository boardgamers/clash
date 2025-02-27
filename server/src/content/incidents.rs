use crate::content::incidents_1::{good_year, pestilence};
use crate::content::incidents_2::population_boom;
use crate::content::incidents_5::successful_year;
use crate::incident::Incident;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<Incident> {
    let all = vec![
        // 1+
        pestilence(),
        good_year(),
        // 11+
        population_boom(),
        // 51+
        successful_year(),
    ]
    .into_iter()
    .flatten()
    .collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| i.id).count(),
        all.len(),
        "Incident ids are not unique"
    );
    all
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_incident(id: u8) -> Incident {
    get_all()
        .into_iter()
        .find(|incident| incident.id == id)
        .expect("incident not found")
}
