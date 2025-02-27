use crate::content::custom_phase_actions::ResourceRewardRequest;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;

pub(crate) fn successful_year() -> Vec<Incident> {
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
