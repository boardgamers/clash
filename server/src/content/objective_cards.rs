use crate::content::objectives::combat_objectives::{
    aggressor, barbarian_conquest, bold, brutus, conqueror, defiance, general, great_battle,
    great_commander, legendary_battle, naval_assault, resistance, scavenger, warmonger,
};
use crate::content::objectives::non_combat::{
    city_founder, draft, magnificent_culture, terror_regime,
};

use crate::cache;
use crate::content::objectives::advance_objectives::{
    city_planner, culture_focus, diversified_research, education_lead, goal_focused, government,
    militarized, science_focus, seafarers, trade_focus,
};
use crate::content::objectives::city_objectives::{
    advanced_culture, architecture, coastal_lead, consulate, culture_power, expansionist,
    fortifications, happy_population, large_civ, legacy, metropolis, religious_fervor,
    science_lead, star_gazers, traders,
};
use crate::content::objectives::resource_objectives::{
    eureka, food_supplies, optimized_storage, ore_supplies, wealth, wood_supplies,
};
use crate::content::objectives::unit_objectives::{
    colony, horse_power, ivory_tower, large_army, large_fleet, military_might, outpost,
    sea_blockade, shipping_routes, standing_army, threat, trade_power, versatility,
};
use crate::objective_card::ObjectiveCard;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> &'static Vec<ObjectiveCard> {
    cache::get().get_objective_cards()
}

#[must_use]
pub(crate) fn get_all_uncached() -> Vec<ObjectiveCard> {
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
        ObjectiveCard::new(15, architecture(), militarized()),
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
        ObjectiveCard::new(27, city_founder(), resistance()),
        ObjectiveCard::new(28, trade_power(), resistance()),
        ObjectiveCard::new(29, shipping_routes(), terror_regime()),
        ObjectiveCard::new(30, diversified_research(), bold()),
        ObjectiveCard::new(31, culture_power(), barbarian_conquest()),
        ObjectiveCard::new(32, magnificent_culture(), outpost()),
        ObjectiveCard::new(33, star_gazers(), horse_power()),
        ObjectiveCard::new(34, traders(), great_commander()),
        ObjectiveCard::new(35, versatility(), brutus()),
        ObjectiveCard::new(36, legacy(), ivory_tower()),
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
pub fn get_objective_card(id: u8) -> &'static ObjectiveCard {
    cache::get()
        .get_objective_card(id)
        .unwrap_or_else(|| panic!("objective card not found {id}"))
}
