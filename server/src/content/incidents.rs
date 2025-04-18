mod civil_war;
mod earthquake;
pub(crate) mod famine;
mod good_year;
pub(crate) mod great_builders;
pub(crate) mod great_diplomat;
pub(crate) mod great_explorer;
pub(crate) mod great_persons;
pub(crate) mod great_warlord;
mod pandemics;
mod trade;
pub(crate) mod trojan;

use crate::cache;
use crate::content::incidents::civil_war::civil_war_incidents;
use crate::content::incidents::earthquake::earthquake_incidents;
use crate::content::incidents::famine::pestilence_incidents;
use crate::content::incidents::good_year::good_years_incidents;
use crate::content::incidents::great_persons::great_person_incidents;
use crate::content::incidents::pandemics::pandemics_incidents;
use crate::content::incidents::trade::trade_incidents;
use crate::content::incidents::trojan::trojan_incidents;
use crate::incident::Incident;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> &'static Vec<Incident> {
    cache::get().get_incidents()
}

#[must_use]
pub(crate) fn get_all_uncached() -> Vec<Incident> {
    let all = vec![
        // 1+
        pestilence_incidents(),
        // 9+
        good_years_incidents(),
        // 29+
        earthquake_incidents(),
        // 34+
        civil_war_incidents(),
        // 41+
        trojan_incidents(),
        // 45+
        trade_incidents(),
        // 49+
        pandemics_incidents(),
        // 18+
        great_person_incidents(),
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
pub fn get_incident(id: u8) -> &'static Incident {
    cache::get().get_incident(id).expect("incident not found")
}
