use crate::content::objectives::combat_objectives::{conqueror, defiance, general, great_battle, naval_assault, warmonger};
use crate::content::objectives::non_combat::draft;
use crate::content::objectives::status_phase_objectives::{advanced_culture, city_planner, coastal_lead, education_lead, eureka, food_supplies, fortifications, happy_population, large_army, large_civ, large_fleet, optimized_storage, ore_supplies, religious_fervor, science_lead, sea_blockade, standing_army, wealth, wood_supplies};
use crate::objective_card::ObjectiveCard;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<ObjectiveCard> {
    let all = vec![
        ObjectiveCard::new(1, large_civ(), draft()),
        ObjectiveCard::new(2, science_lead(), conqueror()),
        ObjectiveCard::new(3, coastal_lead(), warmonger()),
        ObjectiveCard::new(4, advanced_culture(), warmonger()),
        ObjectiveCard::new(5, religious_fervor(), general()),
        ObjectiveCard::new(6, city_planner(), great_battle()),
        ObjectiveCard::new(7, eureka(), defiance()),
        ObjectiveCard::new(8, happy_population(), conqueror()),
        ObjectiveCard::new(9, education_lead(), sea_blockade()),
        ObjectiveCard::new(10, optimized_storage(), naval_assault()),
        ObjectiveCard::new(11, wealth(), large_fleet()),
        ObjectiveCard::new(12, ore_supplies(), large_army()),
        ObjectiveCard::new(13, wood_supplies(), fortifications()),
        ObjectiveCard::new(14, food_supplies(), standing_army()),
        // todo replace when we have a real repeated objective - only needed for large civ
        // todo use ID 24 later
        ObjectiveCard::new(99, large_civ(), draft()),
    ];
    assert_eq!(
        all.iter().unique_by(|i| i.id).count(),
        all.len(),
        "Objective card ids are not unique"
    );
    all
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_objective_card(id: u8) -> ObjectiveCard {
    get_all()
        .into_iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("objective card not found {id}"))
}
