use crate::advance::Advance;
use crate::advance::AdvanceBuilder;
use crate::content::advances_agriculture::agriculture;
use crate::content::advances_autocracy::autocracy;
use crate::content::advances_construction::construction;
use crate::content::advances_culture::culture;
use crate::content::advances_democracy::democracy;
use crate::content::advances_economy::economy;
use crate::content::advances_education::education;
use crate::content::advances_science::science;
use crate::content::advances_seafearing::seafaring;
use crate::content::advances_spirituality::spirituality;
use crate::content::advances_theocracy::theocracy;
use crate::content::advances_warfare::warfare;
use std::vec;

//names of advances that need special handling
pub const NAVIGATION: &str = "Navigation";
pub const ROADS: &str = "Roads";
pub const STEEL_WEAPONS: &str = "Steel Weapons";
pub const METALLURGY: &str = "Metallurgy";
pub const TACTICS: &str = "Tactics";
pub const CURRENCY: &str = "Currency";
pub const IRRIGATION: &str = "Irrigation";

const GOVERNMENTS: [(&str, &str, &str); 3] = [
    ("Democracy", "Voting", "Philosophy"),
    ("Autocracy", "Nationalism", "Draft"),
    ("Theocracy", "Dogma", "State Religion"),
];

pub struct AdvanceGroup {
    pub name: String,
    pub advances: Vec<Advance>,
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
    let a: Vec<Advance> = advances
        .into_iter()
        .enumerate()
        .map(|(index, builder)| {
            if index > 0 {
                assert_eq!(builder.required_advance, None);
                builder.with_required_advance(first)
            } else {
                // first advance in the group
                if let Some((government, _leading, requirement)) = GOVERNMENTS
                    .into_iter()
                    .find(|(_government, leading, _requirement)| leading == first)
                {
                    return builder
                        .with_required_advance(requirement)
                        .leading_government_advance(government)
                        .with_contradicting_advance(
                            &GOVERNMENTS
                                .into_iter()
                                .map(|(_government, leading, _requirement)| leading)
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
    }
}

///
/// # Panics
///
/// Panics if advance with name doesn't exist
#[must_use]
pub fn get_advance_by_name(name: &str) -> Advance {
    get_all()
        .into_iter()
        .find(|advance| advance.name == name)
        .unwrap_or_else(|| {
            panic!("Advance with name {name} not found");
        })
}

pub(crate) fn get_advances_by_group(group: &str) -> Vec<Advance> {
    get_groups()
        .into_iter()
        .find(|g| g.name == group)
        .map_or_else(
            || {
                panic!("Group with name {group} not found");
            },
            |g| g.advances,
        )
}

#[must_use]
pub fn get_leading_government_advance(government: &str) -> Option<Advance> {
    get_all().into_iter().find(|advance| {
        advance
            .government
            .as_ref()
            .is_some_and(|value| value.as_str() == government)
    })
}

#[must_use]
pub fn get_governments() -> Vec<(String, Advance)> {
    get_all()
        .into_iter()
        .filter_map(|advance| advance.government.clone().map(|g| (g.clone(), advance)))
        .collect()
}

///
///
/// # Panics
///
/// Panics if government doesn't have a leading government advance or if some of the government advances do no have their government tier specified
#[must_use]
pub fn get_government(government: &str) -> Vec<Advance> {
    let leading_government =
        get_leading_government_advance(government).expect("government should exist");
    let mut government_advances = get_all()
        .into_iter()
        .filter(|advance| {
            advance
                .required
                .as_ref()
                .is_some_and(|required_advance| required_advance == &leading_government.name)
        })
        .collect::<Vec<Advance>>();
    government_advances.push(leading_government);
    government_advances.reverse();
    government_advances
}
