use crate::content::advances::trade_routes::find_trade_routes;
use crate::content::objectives::city_objectives::leading_player;
use crate::content::objectives::non_combat::last_player_round;
use crate::map::home_position;
use crate::objective_card::Objective;
use crate::player::Player;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn sea_blockade() -> Objective {
    Objective::builder(
        "Sea Blockade",
        "At least 2 of your ships are on the \
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

pub(crate) fn large_fleet() -> Objective {
    Objective::builder(
        "Large Fleet",
        "You have at least 4 ships - or 2 ships and more than any other player.",
    )
    .status_phase_check(|game, player| {
        let ships = ship_count(player);
        ships >= 4 || (ships >= 2 && leading_player(game, player, 1, |p, _| ship_count(p)))
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
    .status_phase_check(|game, player| {
        leading_player(game, player, 4, |p, _| {
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

pub(crate) fn colony() -> Objective {
    Objective::builder(
        "Colony",
        "You have at least 1 city at least 5 spaces away from your starting city position. \
        Cannot be completed if you completed City Founder in the last round.",
    )
    .status_phase_check(|game, player| {
        let home = home_position(game, player);
        if player.cities.iter().any(|c| c.position.distance(home) >= 5) {
            let city_founder_played = last_player_round(game, player.index)
                .iter()
                .any(|i| i.completed_objectives.contains(&"City Founder".to_string()));

            return !city_founder_played;
        }
        false
    })
    .build()
}

pub(crate) fn threat() -> Objective {
    Objective::builder(
        "Threat",
        "At least 4 of your army units are adjacent to another human player's city.",
    )
    .status_phase_check(|game, player| {
        let enemy_cities = game
            .players
            .iter()
            .filter(|p| p.index != player.index && p.is_human())
            .flat_map(|p| p.cities.iter().map(|c| c.position).collect_vec())
            .collect_vec();

        player
            .units
            .iter()
            .filter(|u| {
                u.unit_type.is_army_unit()
                    && u.position
                        .neighbors()
                        .iter()
                        .any(|n| enemy_cities.contains(n))
            })
            .count()
            >= 4
    })
    .build()
}

pub(crate) fn outpost() -> Objective {
    Objective::builder(
        "Outpost",
        "You have army units on at least 3 spaces outside, and not adjacent to cities",
    )
    .status_phase_check(|_game, player| {
        player
            .units
            .iter()
            .filter_map(|u| {
                (u.unit_type.is_army_unit()
                    && player
                        .cities
                        .iter()
                        .all(|c| c.position.distance(u.position) > 1))
                .then_some(u.position)
            })
            .unique()
            .count()
            >= 3
    })
    .build()
}

pub(crate) fn migration() -> Objective {
    Objective::builder(
        "Migration",
        "You have settlers on at least 3 spaces outside, and not adjacent to cities",
    )
    .status_phase_check(|_game, player| {
        player
            .units
            .iter()
            .filter_map(|u| {
                (u.unit_type.is_settler()
                    && player
                        .cities
                        .iter()
                        .all(|c| c.position.distance(u.position) > 1))
                .then_some(u.position)
            })
            .unique()
            .count()
            >= 3
    })
    .build()
}

pub(crate) fn military_might() -> Objective {
    Objective::builder(
        "Military Might",
        "You have at least 12 army units and ships combined. \
        Cannot be completed together with Standing Army.",
    )
    .contradicting_status_phase_objective("Standing Army")
    .status_phase_check(|_game, player| {
        player
            .units
            .iter()
            .filter(|u| u.unit_type.is_military())
            .count()
            >= 12
    })
    .build()
}

pub(crate) fn trade_power() -> Objective {
    Objective::builder(
        "Trade Power",
        "You could form at least 3 trade routes if you wanted to.\
        Cannot be completed together with Shipping Routes.",
    )
    .contradicting_status_phase_objective("Shipping Routes")
    .status_phase_check(|game, player| find_trade_routes(game, player, false).len() >= 3)
    .build()
}

pub(crate) fn shipping_routes() -> Objective {
    Objective::builder(
        "Shipping Routes",
        "You could form at least 2 trade routes only with ships if you wanted to.\
        Cannot be completed together with Trade Power.",
    )
    .contradicting_status_phase_objective("Trade Power")
    .status_phase_check(|game, player| find_trade_routes(game, player, true).len() >= 2)
    .build()
}

pub(crate) fn horse_power() -> Objective {
    unit_versatility("Horse Power", UnitType::Cavalry)
}

pub(crate) fn ivory_tower() -> Objective {
    unit_versatility("Ivory Tower", UnitType::Elephant)
}

pub(crate) fn unit_versatility(objective: &str, unit_type: UnitType) -> Objective {
    Objective::builder(
        objective,
        &format!(
            "You have at least 3 army groups with at least 1 {} unit each.",
            unit_type.name()
        ),
    )
    .status_phase_check(move |_game, player| {
        player
            .units
            .iter()
            .filter(|u| u.unit_type == unit_type)
            .map(|u| u.position)
            .unique()
            .count()
            >= 3
    })
    .build()
}

pub(crate) fn versatility() -> Objective {
    Objective::builder(
        "Versatility",
        "You have at least 1 of each unit \
        (ship, infantry, cavalry, elephant, leader, settler)",
    )
    .status_phase_check(|_game, player| player.units.iter().unique_by(|u| u.unit_type).count() >= 6)
    .build()
}
