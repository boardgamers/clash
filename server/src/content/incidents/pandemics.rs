use crate::city::City;
use crate::content::custom_phase_actions::{
    PaymentRequest, PositionRequest, ResourceRewardRequest, UnitsRequest,
};
use crate::content::incidents::famine::{
    additional_sanitation_damage, famine, kill_incident_units,
};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, MoodModifier};
use crate::map::{Map, Terrain};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::IncidentTarget;
use crate::position::Position;
use crate::resource::ResourceType;
use itertools::Itertools;

pub(crate) fn pandemics_incidents() -> Vec<Incident> {
    vec![
        pandemics(),
        black_death(),
        vermin(),
        draught(),
        fire(),
        successful_year(),
    ]
}

fn pandemics() -> Incident {
    // todo can also discard any cards to pay the cost
    Incident::builder(
        49,
        "Pandemics",
        "Every player loses an amount resources, tokens, cards, \
        or units equal to the half of the number of their cities (rounded down).",
        IncidentBaseEffect::BarbariansMove,
    )
    .set_protection_advance("Sanitation")
    .add_incident_units_request(
        IncidentTarget::AllPlayers,
        1,
        |game, p, _i| {
            let player = game.get_player(p);
            let needed = pandemics_cost(player);
            let choices = player.units.iter().map(|u| u.id).collect_vec();
            let min = needed.saturating_sub(player.resources.amount() as u8);
            let max = needed.min(choices.len() as u8);
            Some(UnitsRequest::new(
                p,
                choices,
                min..=max,
                "Select units to lose",
            ))
        },
        |game, s| {
            kill_incident_units(game, s);
            game.current_event_mut().player.sacrifice = s.choice.len() as u8;
        },
    )
    .add_incident_payment_request(
        IncidentTarget::AllPlayers,
        0,
        |game, p, _i| {
            let player = game.get_player(p);
            let resources = player.resources.amount() as u8;
            let needed =
                (pandemics_cost(player) - game.current_event_mut().player.sacrifice).min(resources);
            Some(vec![PaymentRequest::new(
                PaymentOptions::sum(needed as u32, &ResourceType::all()),
                "Select resources to lose",
                false,
            )])
        },
        |game, s| {
            game.add_info_log_item(&format!("{} lost {}", s.player_name, s.choice[0]));
        },
    )
    .build()
}

fn pandemics_cost(player: &Player) -> u8 {
    (player.cities.len() / 2) as u8
}

fn black_death() -> Incident {
    Incident::builder(
        50,
        "Black Death",
        "Every player with at least 4 units: \
        Lose 1 unit for every 4 units they have (rounded down). \
        If you have Roads, Navigation, or Trade Routes, you lose 1 additional unit. \
        Gain 1 victory point for every unit lost.",
        IncidentBaseEffect::None,
    )
    .add_incident_units_request(
        IncidentTarget::AllPlayers,
        0,
        |game, p, _i| {
            let player = game.get_player(p);
            let units = player.units.iter().map(|u| u.id).collect_vec();
            if units.len() < 4 {
                return None;
            };

            let mut needed = (units.len() / 4) as u8;
            if additional_sanitation_damage(player) {
                needed += 1;
            }

            Some(UnitsRequest::new(
                p,
                units,
                needed..=needed,
                "Select units to lose",
            ))
        },
        |game, s| {
            kill_incident_units(game, s);
            let vp = s.choice.len() as f32;
            game.add_info_log_item(&format!("{} gained {} victory points", s.player_name, vp));
            game.get_player_mut(s.player_index).event_victory_points += vp;
        },
    )
    .build()
}

fn vermin() -> Incident {
    famine(51,
           "Famine: Vermin",
           "Every player with Storage: Pay 1 food (gold not allowed). If you cannot pay, make 1 city Angry.",
           IncidentTarget::AllPlayers,
           IncidentBaseEffect::None,
           |_, _| 1,
           |p| p.has_advance("Storage"),
           |_, _| true,
    )
}

fn draught() -> Incident {
    famine(52,
           "Famine: Draught",
           "Pay 1 food for every city on or adjacent to Barren Land (up to 3 food, gold not allowed). If you cannot pay the full amount, make 1 of those cities Angry.",
           IncidentTarget::ActivePlayer,
           IncidentBaseEffect::None,
           |p, game| p.cities.iter().filter(|c| on_or_adjacent_to_barren(c, game)).count().min(3) as u8,
           |_| true,
           on_or_adjacent_to_barren,
    )
}

fn on_or_adjacent_to_barren(c: &City, game: &Game) -> bool {
    game.map.get(c.position) == Some(&Terrain::Barren)
        || c.position
            .neighbors()
            .iter()
            .any(|p| game.map.get(*p) == Some(&Terrain::Barren))
}

fn fire() -> Incident {
    Incident::builder(
        53,
        "Fire",
        "Select one of your cities that is placed on a Forest. \
        Decrease the mood in that city, and all cities adjacent that are part of the same forest, \
        regardless of who owns them. If you have no cities on a Forest, loose 1 wood.",
        IncidentBaseEffect::GoldDeposits,
    )
    .add_incident_position_request(
        IncidentTarget::ActivePlayer,
        11,
        |game, p, _i| {
            let player = game.get_player(p);
            let cities = player
                .cities
                .iter()
                .filter(|c| game.map.get(c.position) == Some(&Terrain::Forest))
                .map(|c| c.position)
                .collect_vec();
            let name = game.player_name(p);
            if cities.is_empty() {
                if player.resources.wood > 0 {
                    game.add_info_log_item(&format!("{name} lost 1 wood"));
                    return None;
                }
                game.add_info_log_item(&format!(
                    "{name} has no cities on a Forest and no wood to lose"
                ));
                return None;
            }
            Some(PositionRequest::new(
                cities,
                1..=1,
                "Select a city to set on fire",
            ))
        },
        |game, s| {
            game.current_event_mut().selected_position = Some(s.choice[0]);
        },
    )
    .add_decrease_mood(
        IncidentTarget::AllPlayers,
        MoodModifier::Decrease,
        |p, game| {
            let b = burning_cities(p, game);
            let a = b.len() as u8;
            (b, a)
        },
    )
    .build()
}

fn burning_cities(p: &Player, game: &Game) -> Vec<Position> {
    if let Some(pos) = game.current_event().selected_position {
        let mut fire = vec![];
        spread_fire(pos, &game.map, &mut fire);
        p.cities
            .iter()
            .filter(|c| fire.contains(&c.position))
            .map(|c| c.position)
            .collect_vec()
    } else {
        vec![]
    }
}

fn spread_fire(p: Position, map: &Map, fire: &mut Vec<Position>) {
    if fire.contains(&p) {
        return;
    }
    if map.get(p) == Some(&Terrain::Forest) {
        fire.push(p);
        for n in p.neighbors() {
            spread_fire(n, map, fire);
        }
    }
}

pub(crate) fn successful_year() -> Incident {
    Incident::builder(
        54,
        "A successful year",
        "All players with the fewest cities gains 1 food for every city \
        they have less than the player with the most cities. \
        If everyone has the same number of cities, all players gain 1 food.",
        IncidentBaseEffect::PiratesSpawnAndRaid,
    )
    .add_incident_resource_request(
        IncidentTarget::AllPlayers,
        0,
        |game, player_index, _incident| {
            let player_to_city_num = game
                .players
                .iter()
                .map(|p| p.cities.len())
                .collect::<Vec<_>>();

            let min_cities = player_to_city_num.iter().min().unwrap_or(&0);
            let max_cities = player_to_city_num.iter().max().unwrap_or(&0);
            if min_cities == max_cities {
                return Some(ResourceRewardRequest::new(
                    PaymentOptions::sum(1, &[ResourceType::Food]),
                    "-".to_string(),
                ));
            }

            let cities = game.players[player_index].cities.len();
            if cities == *min_cities {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::sum((max_cities - min_cities) as u32, &[ResourceType::Food]),
                    "-".to_string(),
                ))
            } else {
                None
            }
        },
        |_game, s| {
            vec![format!(
                "{} gained {} from A successful year",
                s.player_name, s.choice
            )]
        },
    )
    .build()
}
