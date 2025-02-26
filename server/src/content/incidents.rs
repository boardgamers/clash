use crate::content::custom_phase_actions::{PlayerRequest, PositionRequest, ResourceRewardRequest};
use crate::game::Game;
use crate::incident::{Incident, IncidentBaseEffect, IncidentBuilder};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;
use crate::unit::UnitType;
use itertools::Itertools;
use std::vec;

#[must_use]
pub(crate) fn get_all() -> Vec<Incident> {
    let all = vec![good_year(), population_boom(), successful_year()]
        .into_iter()
        .flatten()
        .collect_vec();
    assert_eq!(
        all.iter().unique_by(|i| i.id).count(),
        all.len(),
        "Incident ids are not unique"
    );
    all
}

fn population_boom() -> Vec<Incident> {
    let mut b = Incident::builder(
        28,
        "Population Boom",
        "-",
        IncidentBaseEffect::BarbariansMove,
    );
    b = select_settler(b, 2, |game, _| {
        game.current_custom_phase().is_active_player()
    });
    b = b.add_incident_player_request(
        IncidentTarget::ActivePlayer,
        1,
        |game, player_index, _incident| {
            let choices = game
                .players
                .iter()
                .filter(|p| p.available_units().settlers > 0 && p.index != player_index)
                .map(|p| p.index)
                .collect_vec();

            if choices.is_empty() {
                None
            } else {
                Some(PlayerRequest::new(
                    choices,
                    "Select a unit to gain 1 settler",
                ))
            }
        },
        |game, c| {
            game.add_info_log_item(&format!(
                "{} was selected to gain 1 settler from Population Boom",
                game.get_player(c.choice).get_name()
            ));
            game.current_custom_phase_mut().selected_player = Some(c.choice);
        },
    );
    b = select_settler(b, 0, |game, player| {
        let c = game.current_custom_phase();
        c.selected_player == Some(player)
    });
    vec![b.build()]
}

fn select_settler(
    b: IncidentBuilder,
    priority: i32,
    pred: impl Fn(&Game, usize) -> bool + 'static + Clone,
) -> IncidentBuilder {
    b.add_incident_position_request(
        IncidentTarget::AllPlayers,
        priority,
        move |game, player_index, _incident| {
            let p = game.get_player(player_index);
            if pred(game, player_index) && p.available_units().settlers > 0 {
                Some(PositionRequest::new(
                    p.cities.iter().map(|c| c.position).collect(),
                    Some("Select a city to gain 1 settler".to_string()),
                ))
            } else {
                None
            }
        },
        |game, s| {
            game.add_info_log_item(&format!(
                "{} gained 1 settler in {}",
                s.player_name, s.choice
            ));
            game.get_player_mut(s.player_index)
                .add_unit(s.choice, UnitType::Settler);
        },
    )
}

fn successful_year() -> Vec<Incident> {
    vec![Incident::builder(
        54,
        "A successful year",
        "-",
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
    .build()]
}

fn good_year() -> Vec<Incident> {
    vec![
        add_good_year(
            IncidentTarget::AllPlayers,
            Incident::builder(
                9,
                "A good year",
                "Every player gains 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
        ),
        add_good_year(
            IncidentTarget::ActivePlayer,
            Incident::builder(
                10,
                "A good year",
                "You gain 1 food",
                IncidentBaseEffect::BarbariansSpawn,
            ),
        ),
    ]
}

fn add_good_year(target: IncidentTarget, builder: IncidentBuilder) -> Incident {
    builder
        .add_incident_resource_request(
            target,
            0,
            |_game, _player_index, _incident| {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::sum(1, &[ResourceType::Food]),
                    "Gain 1 food".to_string(),
                ))
            },
            |_game, s| {
                vec![format!(
                    "{} gained {} from A good year",
                    s.player_name, s.choice
                )]
            },
        )
        .build()
}

///
/// # Panics
/// Panics if incident does not exist
#[must_use]
pub fn get_incident(id: u8) -> Incident {
    get_all()
        .into_iter()
        .find(|incident| incident.id == id)
        .expect("incident not found")
}
