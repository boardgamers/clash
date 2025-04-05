mod agriculture;
mod autocracy;
mod construction;
pub mod culture;
mod democracy;
pub mod economy;
mod education;
mod science;
mod seafaring;
mod spirituality;
pub(crate) mod theocracy;
pub mod trade_routes;
pub(crate) mod  warfare;

use crate::advance::Advance;
use crate::advance::AdvanceBuilder;
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
use std::vec;

//names of advances that need special handling
pub const NAVIGATION: &str = "Navigation";
pub const ROADS: &str = "Roads";
pub const STEEL_WEAPONS: &str = "Steel Weapons";
pub const METALLURGY: &str = "Metallurgy";
pub const TACTICS: &str = "Tactics";
pub const CURRENCY: &str = "Currency";
pub const IRRIGATION: &str = "Irrigation";

struct GovernmentInfo {
    name: &'static str,
    leading: &'static str,
    requirement: &'static str,
}

const GOVERNMENTS: [GovernmentInfo; 3] = [
    GovernmentInfo {
        name: "Democracy",
        leading: "Voting",
        requirement: "Philosophy",
    },
    GovernmentInfo {
        name: "Autocracy",
        leading: "Nationalism",
        requirement: "Draft",
    },
    GovernmentInfo {
        name: "Theocracy",
        leading: "Dogma",
        requirement: "State Religion",
    },
];

pub struct AdvanceGroup {
    pub name: String,
    pub advances: Vec<Advance>,
    pub government: Option<String>,
}

#[must_use]
pub fn get_all() -> Vec<Advance> {
    get_groups().into_iter().flat_map(|g| g.advances).collect()
}

#[must_use]
pub fn get_groups() -> Vec<AdvanceGroup> {
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
    let first = &advances[0].name.clone();
    let government = GOVERNMENTS.into_iter().find(|i| first == i.leading);
    let a: Vec<Advance> = advances
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
                            &GOVERNMENTS
                                .into_iter()
                                .map(|i| i.leading)
                                .collect::<Vec<&str>>(),
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

///
/// # Panics
///
/// Panics if advance with name doesn't exist
#[must_use]
pub fn get_advance(name: &str) -> Advance {
    get_all()
        .into_iter()
        .find(|advance| advance.name == name)
        .unwrap_or_else(|| {
            panic!("Advance with name {name} not found");
        })
}

pub(crate) fn get_group(group: &str) -> AdvanceGroup {
    get_groups()
        .into_iter()
        .find(|g| g.name == group)
        .expect("Group not found")
}

#[must_use]
pub fn get_governments() -> Vec<AdvanceGroup> {
    get_groups()
        .into_iter()
        .filter(|g| g.government.is_some())
        .collect::<Vec<AdvanceGroup>>()
}

///
///
/// # Panics
///
/// Panics if government doesn't have a leading government advance or if some of the government advances do no have their government tier specified
#[must_use]
pub fn get_government(government: &str) -> Option<AdvanceGroup> {
    get_groups().into_iter().find(|g| {
        g.government
            .as_ref()
            .is_some_and(|value| value.as_str() == government)
    })
}
