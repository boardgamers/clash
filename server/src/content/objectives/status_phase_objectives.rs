use crate::city::MoodState;
use crate::city_pieces::Building;
use crate::content::advances;
use crate::game::Game;
use crate::objective_card::{Objective, ObjectiveBuilder};
use crate::player::Player;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub(crate) fn large_civ() -> Objective {
    Objective::builder("Large Civilization", "You have at least 6 cities")
        .status_phase_check(|_game, player| player.cities.len() >= 6)
        .build()
}

pub(crate) fn science_lead() -> Objective {
    building_lead(
        Objective::builder(
            "Scientific Lead",
            "You have more academies than any other player",
        ),
        Building::Academy,
    )
    .build()
}

pub(crate) fn coastal_lead() -> Objective {
    building_lead(
        Objective::builder(
            "Coastal Culture",
            "You have more ports than any other player",
        ),
        Building::Port,
    )
    .build()
}

pub(crate) fn religious_fervor() -> Objective {
    building_lead(
        Objective::builder(
            "Religious Fervor",
            "You have more temples than any other player",
        ),
        Building::Temple,
    )
    .build()
}

pub(crate) fn fortifications() -> Objective {
    building_lead(
        Objective::builder(
            "Fortifications",
            "You have more fortresses than any other player",
        ),
        Building::Fortress,
    )
    .build()
}

fn building_lead(b: ObjectiveBuilder, building: Building) -> ObjectiveBuilder {
    b.status_phase_check(move |game, player| {
        leading_player(game, player, 1, move |p| buildings(p, building))
    })
}

fn leading_player(
    game: &Game,
    player: &Player,
    margin: usize,
    value: impl Fn(&Player) -> usize + 'static,
) -> bool {
    value(player)
        >= game
            .players
            .iter()
            .filter(|p| p.index != player.index && p.is_human())
            .map(value)
            .max()
            .unwrap_or(0)
            + margin
}

fn buildings(p: &Player, b: Building) -> usize {
    p.cities
        .iter()
        .filter(|c| c.pieces.building_owner(b).is_some())
        .count()
}

pub(crate) fn advanced_culture() -> Objective {
    Objective::builder(
        "Advanced Culture",
        "You have more advances than any other player - at least 6.",
    )
    .status_phase_check(|game, player| {
        player.advances.len() >= 6 && leading_player(game, player, 1, move |p| p.advances.len())
    })
    .build()
}

fn advance_group_complete(objective: &str, group: &'static str) -> Objective {
    Objective::builder(objective, &format!("You have all {group} advances."))
        .status_phase_check(move |_game, player| {
            advances::get_group(group)
                .advances
                .iter()
                .all(|a| player.has_advance(&a.name))
        })
        .build()
}

pub(crate) fn city_planner() -> Objective {
    advance_group_complete("City Planner", "Construction")
}

pub(crate) fn education_lead() -> Objective {
    advance_group_complete("Education Lead", "Education")
}

pub(crate) fn militarized() -> Objective {
    advance_group_complete("Militarized", "Warfare")
}

pub(crate) fn eureka() -> Objective {
    supplies("Eureka!", ResourceType::Ideas)
}

pub(crate) fn supplies(objective: &'static str, r: ResourceType) -> Objective {
    pay_resources(objective, ResourcePile::of(r, 5), ResourcePile::of(r, 2))
}

pub(crate) fn pay_resources(
    objective: &'static str,
    want: ResourcePile,
    pay: ResourcePile,
) -> Objective {
    let suffix = if pay.gold > 0 { " (not gold)" } else { "" };
    Objective::builder(
        objective,
        &format!("You have at least {want}: Pay {pay}{suffix}."),
    )
    .status_phase_check(move |_game, player| player.resources.has_at_least(&want))
    .status_phase_update(move |game, player| {
        game.player_mut(player).lose_resources(pay.clone());
        game.add_info_log_item(&format!(
            "{} paid {pay} for {objective}",
            game.player_name(player)
        ));
    })
    .build()
}

pub(crate) fn happy_population() -> Objective {
    Objective::builder("Happy Population", "You have at least 4 happy cities.")
        .status_phase_check(|_game, player| {
            player
                .cities
                .iter()
                .filter(|c| c.mood_state == MoodState::Happy)
                .count()
                >= 4
        })
        .build()
}

pub(crate) fn sea_blockade() -> Objective {
    Objective::builder(
        "Sea Blockade",
        "At least 2 of your cities are on the \
    port location of another player",
    )
    .status_phase_check(|game, player| {
        let enemy_ports = game
            .players
            .iter()
            .flat_map(|p| {
                p.cities
                    .iter()
                    .filter_map(|c| if p.is_human() { c.port_position } else { None })
            })
            .collect_vec();

        player
            .units
            .iter()
            .filter(|u| enemy_ports.contains(&u.position))
            .count()
            >= 2
    })
    .build()
}

pub(crate) fn optimized_storage() -> Objective {
    Objective::builder(
        "Optimized Storage",
        "You have at least 3 food, 3 ore, and 3 wood.",
    )
    .status_phase_check(|_game, player| {
        let r = &player.resources;
        r.food >= 3 && r.ore >= 3 && r.wood >= 3
    })
    .build()
}

pub(crate) fn wealth() -> Objective {
    supplies("Wealth", ResourceType::Gold)
}

pub(crate) fn ore_supplies() -> Objective {
    supplies("Ore Supplies", ResourceType::Ore)
}

pub(crate) fn wood_supplies() -> Objective {
    supplies("Wood Supplies", ResourceType::Wood)
}

pub(crate) fn food_supplies() -> Objective {
    supplies("Food Supplies", ResourceType::Food)
}

pub(crate) fn large_fleet() -> Objective {
    Objective::builder(
        "Large Fleet",
        "You have at least 4 ships - or 2 ships and more than any other player.",
    )
    .status_phase_check(|_game, player| {
        let ships = ship_count(player);
        ships >= 4 || (ships >= 2 && leading_player(_game, player, 1, ship_count))
    })
    .build()
}

fn ship_count(p: &Player) -> usize {
    p.units.iter().filter(|u| u.unit_type.is_ship()).count()
}

pub(crate) fn large_army() -> Objective {
    Objective::builder(
        "Large Army",
        "You have at least 4 more army units than any other player.",
    )
    .status_phase_check(|_game, player| {
        leading_player(_game, player, 4, |p| {
            p.units
                .iter()
                .filter(|u| u.unit_type.is_army_unit())
                .count()
        })
    })
    .build()
}

pub(crate) fn standing_army() -> Objective {
    Objective::builder(
        "Standing Army",
        "You have at least 4 cities with army units. \
        Cannot be completed together with Military Might.",
    )
    // todo test when Military Might is implemented
    .contradicting_status_phase_objective("Military Might")
    .status_phase_check(|_game, player| {
        player
            .cities
            .iter()
            .filter(|c| {
                player
                    .get_units(c.position)
                    .iter()
                    .any(|u| u.unit_type.is_army_unit())
            })
            .count()
            >= 4
    })
    .build()
}

pub(crate) fn diversity() -> Objective {
    Objective::builder(
        "Diversity",
        "You have at least 4 different types of buildings \
        (that are not influenced by another player).",
    )
    .status_phase_check(|_game, player| {
        player
            .cities
            .iter()
            .flat_map(|c| c.pieces.buildings(Some(player.index)))
            .unique()
            .count()
            >= 4
    })
    .build()
}

pub(crate) fn goal_focused() -> Objective {
    Objective::builder(
        "Goal Focused",
        "You have more complete advance groups than any other player.",
    )
    .status_phase_check(|_game, player| {
        leading_player(_game, player, 1, |p| {
            advances::get_groups()
                .iter()
                .filter(|g| g.advances.iter().all(|a| p.has_advance(&a.name)))
                .count()
        })
    })
    .build()
}
