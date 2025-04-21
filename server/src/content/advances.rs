mod agriculture;
pub(crate) mod autocracy;
mod construction;
pub mod culture;
pub(crate) mod democracy;
pub mod economy;
mod education;
mod science;
mod seafaring;
mod spirituality;
pub(crate) mod theocracy;
pub mod trade_routes;
pub(crate) mod warfare;

use crate::advance::AdvanceInfo;
use crate::advance::{Advance, AdvanceBuilder};
use crate::content::advances::agriculture::agriculture;
use crate::content::advances::autocracy::autocracy;
use crate::content::advances::construction::construction;
use crate::content::advances::culture::culture;
use crate::content::advances::democracy::democracy;
use crate::content::advances::economy::economy;
use crate::content::advances::education::education;
use crate::content::advances::science::science;
use crate::content::advances::seafaring::seafaring;
use crate::content::advances::spirituality::spirituality;
use crate::content::advances::theocracy::theocracy;
use crate::content::advances::warfare::warfare;
use itertools::Itertools;
use std::vec;

struct GovernmentInfo {
    name: &'static str,
    leading: Advance,
    requirement: Advance,
}

const GOVERNMENTS: [GovernmentInfo; 3] = [
    GovernmentInfo {
        name: "Democracy",
        leading: Advance::Voting,
        requirement: Advance::Philosophy,
    },
    GovernmentInfo {
        name: "Autocracy",
        leading: Advance::Nationalism,
        requirement: Advance::Draft,
    },
    GovernmentInfo {
        name: "Theocracy",
        leading: Advance::Dogma,
        requirement: Advance::StateReligion,
    },
];

#[derive(Clone)]
pub struct AdvanceGroup {
    pub name: String,
    pub advances: Vec<AdvanceInfo>,
    pub government: Option<String>,
}

#[must_use]
pub(crate) fn get_all_uncached() -> Vec<AdvanceInfo> {
    get_groups_uncached()
        .into_iter()
        .flat_map(|g| g.advances)
        .collect()
}

#[must_use]
pub fn get_groups_uncached() -> Vec<AdvanceGroup> {
    vec![
        agriculture(),
        construction(),
        seafaring(),
        education(),
        warfare(),
        spirituality(),
        // second half of the advances
        economy(),
        culture(),
        science(),
        democracy(),
        autocracy(),
        theocracy(),
    ]
}

pub(crate) fn advance_group_builder(name: &str, advances: Vec<AdvanceBuilder>) -> AdvanceGroup {
    let first = advances[0].advance;
    let government = GOVERNMENTS.into_iter().find(|i| first == i.leading);
    let a: Vec<AdvanceInfo> = advances
        .into_iter()
        .enumerate()
        .map(|(index, builder)| {
            let builder = if let Some(g) = &government {
                builder.with_government(g.name)
            } else {
                builder
            };
            if index > 0 {
                assert_eq!(builder.required_advance, None);
                builder.with_required_advance(first)
            } else {
                // first advance in the group
                if let Some(i) = &government {
                    return builder
                        .with_required_advance(i.requirement)
                        .with_contradicting_advance(
                            &GOVERNMENTS.into_iter().map(|i| i.leading).collect_vec(),
                        );
                }
                builder
            }
        })
        .map(AdvanceBuilder::build)
        .collect();
    AdvanceGroup {
        name: name.to_string(),
        advances: a,
        government: government.map(|i| i.name.to_string()),
    }
}

pub(crate) fn get_group_uncached(group: &str) -> AdvanceGroup {
    get_groups_uncached()
        .into_iter()
        .find(|g| g.name == group)
        .unwrap_or_else(|| panic!("Advance group {group} not found"))
}

#[must_use]
pub fn get_governments_uncached() -> Vec<AdvanceGroup> {
    get_groups_uncached()
        .into_iter()
        .filter(|g| g.government.is_some())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::Cache;


    #[test]
    fn test_get_all() {
        let cache = Cache::new();
        let all = cache.get_advances();
        assert!(!all.is_empty());
        let unsorted = all.iter().map(|a| a.advance).collect_vec();

        let sorted = unsorted
            .clone()
            .into_iter()
            .sorted_by_key(|a| *a as usize)
            .collect_vec();
        assert_eq!(sorted, unsorted);
        for advance in all {
            assert_eq!(cache.get_advance(advance.advance).advance, advance.advance);
        }
    }

    #[test]
    fn test_get_groups() {
        let groups = get_groups_uncached();
        assert!(!groups.is_empty());
        assert_eq!(groups.len(), 12);
        assert_eq!(groups[0].name, "Agriculture");
        assert_eq!(groups[5].name, "Spirituality");
    }

    #[test]
    fn test_get_governments() {
        let governments = get_governments_uncached();
        assert!(!governments.is_empty());
        assert_eq!(governments.len(), 3);
        assert_eq!(governments[0].name, "Democracy");
        assert_eq!(governments[2].name, "Theocracy");
    }
}
