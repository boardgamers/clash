use crate::content::incidents_5::successful_year;
use crate::content::incidents_earthquake::earthquakes;
use crate::content::incidents_famine::{epidemics, famines, pestilence};
use crate::content::incidents_good_year::{awesome_years, fantastic_years, good_years};
use crate::content::incidents_population_boom::population_booms;
use crate::incident::Incident;
use itertools::Itertools;
use std::vec;
use crate::content::incidects_civil_war::migrations;

#[must_use]
pub(crate) fn get_all() -> Vec<Incident> {
    let all = vec![
        // 1+
        pestilence(),
        epidemics(),
        famines(),
        // 9+
        good_years(),
        awesome_years(),
        fantastic_years(),
        // 18+
        // great persons
        // 27+
        population_booms(),
        // 29+
        earthquakes(),
        // 34+
        migrations(),
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
