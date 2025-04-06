use crate::content::objectives::combat_objectives::{
    aggressor, barbarian_conquest, bold, conqueror, defiance, general, great_battle,
    legendary_battle, naval_assault, scavenger, warmonger,
};
use crate::content::objectives::non_combat::draft;
use crate::content::objectives::status_phase_objectives::{advanced_culture, city_planner, coastal_lead, colony, consulate, culture_focus, diversity, education_lead, eureka, expansionist, food_supplies, fortifications, goal_focused, government, happy_population, large_army, large_civ, large_fleet, metropolis, militarized, military_might, optimized_storage, ore_supplies, religious_fervor, science_focus, science_lead, sea_blockade, seafarers, standing_army, threat, trade_focus, wealth, wood_supplies};
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
        ObjectiveCard::new(15, diversity(), militarized()),
        ObjectiveCard::new(16, goal_focused(), bold()),
        ObjectiveCard::new(17, colony(), threat()),
        ObjectiveCard::new(18, culture_focus(), legendary_battle()),
        ObjectiveCard::new(19, consulate(), great_battle()),
        ObjectiveCard::new(20, science_focus(), naval_assault()),
        ObjectiveCard::new(21, trade_focus(), scavenger()),
        ObjectiveCard::new(22, metropolis(), general()),
        ObjectiveCard::new(23, seafarers(), aggressor()),
        ObjectiveCard::new(24, government(), barbarian_conquest()),
        ObjectiveCard::new(25, government(), aggressor()),
        ObjectiveCard::new(26, expansionist(), military_might()),
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
