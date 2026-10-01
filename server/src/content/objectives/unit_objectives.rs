use crate::card::HandCardLocation;
use crate::content::advances::trade_routes::find_trade_routes;
use crate::content::objectives::city_objectives::leading_progress;
use crate::content::objectives::non_combat::last_player_round;
use crate::log::ActionLogEntry;
use crate::map::capital_city_position;
use crate::objective_card::{Objective, ObjectiveProgress};
use crate::player::Player;
use crate::unit::UnitType;
use itertools::Itertools;

pub(crate) fn sea_blockade() -> Objective {
    Objective::builder(
        "Sea Blockade",
        "At least 2 of your ships are on the \
        port location of another player",
    )
    .status_phase_progress(|game, player| {
        let enemy_ports = game
            .players
            .iter()
            .flat_map(|p| {
                p.cities
                    .iter()
                    .filter_map(|c| if p.is_human() { c.port_position } else { None })
            })
            .collect_vec();

        vec![ObjectiveProgress::new(
            "Units at port locations",
            player
                .units
                .iter()
                .filter(|u| enemy_ports.contains(&u.position))
                .count(),
            2,
        )]
    })
    .build()
}

pub(crate) fn large_fleet() -> Objective {
    Objective::builder(
        "Large Fleet",
        "You have at least 4 ships, OR at least 2 ships and more ships than every other player.",
    )
    .status_phase_progress(|game, player| {
        let mut progress = leading_progress(
            "Ships · 4, or at least 2 and lead by 1",
            game,
            player,
            1,
            |p, _| ship_count(p),
        );
        progress.target = progress.target.clamp(2, 4);
        vec![progress]
    })
    .build()
}

fn ship_count(p: &Player) -> usize {
    p.units.iter().filter(|u| u.is_ship()).count()
}

pub(crate) fn large_army() -> Objective {
    Objective::builder(
        "Large Army",
        "You have at least 4 more army units than any other player.",
    )
    .status_phase_progress(|game, player| {
        vec![leading_progress(
            "Army units · lead by 4",
            game,
            player,
            4,
            |p, _| p.units.iter().filter(|u| u.is_army_unit()).count(),
        )]
    })
    .build()
}

pub(crate) fn standing_army() -> Objective {
    Objective::builder(
        "Standing Army",
        "At least 4 of your cities each contain one or more of your army units. \
        Cannot be completed together with Military Might.",
    )
    .contradicting_status_phase_objective("Military Might")
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new(
            "Cities with your army",
            player
                .cities
                .iter()
                .filter(|c| {
                    player
                        .get_units(c.position)
                        .iter()
                        .any(|u| u.is_army_unit())
                })
                .count(),
            4,
        )]
    })
    .build()
}

pub(crate) fn colony() -> Objective {
    Objective::builder(
        "Colony",
        "You own a city at least 5 spaces from your starting city position. \
        Cannot be completed if you completed City Founder in the last round.",
    )
    .progress(|game, player| {
        let home = capital_city_position(game, player);
        vec![ObjectiveProgress::new(
            "Farthest city from your start",
            player
                .cities
                .iter()
                .map(|c| c.position.distance(home) as usize)
                .max()
                .unwrap_or(0),
            5,
        )]
    })
    .status_phase_check(|game, player| {
        let home = capital_city_position(game, player);
        if player.cities.iter().any(|c| c.position.distance(home) >= 5) {
            let city_founder_played = last_player_round(game, player.index).iter().any(|a| {
                a.items.iter().any(|i| {
                    if let ActionLogEntry::HandCard {
                        to: HandCardLocation::CompleteObjective(o),
                        ..
                    } = &i.entry
                    {
                        o == "City Founder"
                    } else {
                        false
                    }
                })
            });
            return !city_founder_played;
        }
        false
    })
    .build()
}

pub(crate) fn threat() -> Objective {
    Objective::builder(
        "Threat",
        "At least 4 of your army units are adjacent to cities owned by other players. They may be next to different cities.",
    )
    .status_phase_progress(|game, player| {
        let enemy_cities = game
            .players
            .iter()
            .filter(|p| p.index != player.index && p.is_human())
            .flat_map(|p| p.cities.iter().map(|c| c.position).collect_vec())
            .collect_vec();

        vec![ObjectiveProgress::new("Army units next to other cities", player
            .units
            .iter()
            .filter(|u| {
                u.is_army_unit()
                    && u.position
                        .neighbors()
                        .iter()
                        .any(|n| enemy_cities.contains(n))
            })
            .count(), 4)]
    })
    .build()
}

pub(crate) fn outpost() -> Objective {
    Objective::builder(
        "Outpost",
        "Your army units occupy at least 3 different spaces, each outside and not adjacent to any of your cities.",
    )
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new("Army-occupied spaces away from your cities", player
            .units
            .iter()
            .filter_map(|u| {
                (u.is_army_unit()
                    && player
                        .cities
                        .iter()
                        .all(|c| c.position.distance(u.position) > 1))
                .then_some(u.position)
            })
            .unique()
            .count(), 3)]
    })
    .build()
}

pub(crate) fn migration() -> Objective {
    Objective::builder(
        "Migration",
        "Your settlers occupy at least 3 different spaces, each outside and not adjacent to any of your cities.",
    )
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new("Settler-occupied spaces away from your cities", player
            .units
            .iter()
            .filter_map(|u| {
                (u.is_settler()
                    && player
                        .cities
                        .iter()
                        .all(|c| c.position.distance(u.position) > 1))
                .then_some(u.position)
            })
            .unique()
            .count(), 3)]
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
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new(
            "Army units and ships",
            player.units.iter().filter(|u| u.is_military()).count(),
            12,
        )]
    })
    .build()
}

pub(crate) fn trade_power() -> Objective {
    Objective::builder(
        "Trade Power",
        "You could form at least 3 trade routes. \
        Cannot be completed together with Shipping Routes.",
    )
    .contradicting_status_phase_objective("Shipping Routes")
    .status_phase_progress(|game, player| {
        vec![ObjectiveProgress::new(
            "Possible trade routes",
            find_trade_routes(game, player, false).len(),
            3,
        )]
    })
    .build()
}

pub(crate) fn shipping_routes() -> Objective {
    Objective::builder(
        "Shipping Routes",
        "You could form at least 2 trade routes using your ships. \
        Cannot be completed together with Trade Power.",
    )
    .contradicting_status_phase_objective("Trade Power")
    .status_phase_progress(|game, player| {
        vec![ObjectiveProgress::new(
            "Possible ship trade routes",
            find_trade_routes(game, player, true).len(),
            2,
        )]
    })
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
            "You have at least 1 {} on each of at least 3 different spaces.",
            unit_type.non_leader_name()
        ),
    )
    .status_phase_progress(move |_game, player| {
        vec![ObjectiveProgress::new(
            format!("Spaces with {}", unit_type.non_leader_name()),
            player
                .units
                .iter()
                .filter(|u| u.unit_type == unit_type)
                .map(|u| u.position)
                .unique()
                .count(),
            3,
        )]
    })
    .build()
}

pub(crate) fn versatility() -> Objective {
    Objective::builder(
        "Versatility",
        "You have at least 1 of each unit \
        (ship, infantry, cavalry, elephant, leader, settler)",
    )
    .status_phase_progress(|_game, player| {
        vec![ObjectiveProgress::new(
            "Different unit types",
            player.units.iter().unique_by(|u| u.unit_type).count(),
            6,
        )]
    })
    .build()
}
