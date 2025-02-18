use crate::content::custom_phase_actions::CustomPhaseResourceRewardRequest;
use crate::incident::{Incident, IncidentBaseEffect};
use crate::payment::PaymentOptions;
use crate::player_events::IncidentTarget;
use crate::resource::ResourceType;

#[must_use]
pub fn get_all() -> Vec<Incident> {
    vec![good_year()].into_iter().flatten().collect()
}

fn good_year() -> Vec<Incident> {
    vec![
        Incident::builder(
            9,
            "A good year",
            "Every player gains 1 food",
            IncidentBaseEffect::BarbariansSpawn,
        )
        .add_incident_resource_listener(
            IncidentTarget::AllPlayers,
            1,
            |_game, _player_index, _incident| {
                Some(CustomPhaseResourceRewardRequest {
                    reward: PaymentOptions::sum(1, &[ResourceType::Food]),
                    name: "Gain 1 food".to_string(),
                })
            },
            |_game, _player_index, player_name, resource, _selected| {
                format!("{player_name} gained {resource} from A good year")
            },
        )
        .build(),
        Incident::builder(
            10,
            "A good year",
            "You gain 1 food",
            IncidentBaseEffect::BarbariansSpawn,
        )
        .add_incident_resource_listener(
            IncidentTarget::ActivePlayer,
            1,
            |_game, _player_index, _incident| {
                Some(CustomPhaseResourceRewardRequest {
                    reward: PaymentOptions::sum(1, &[ResourceType::Food]),
                    name: "Gain 1 food".to_string(),
                })
            },
            |_game, _player_index, player_name, resource, _selected| {
                format!("{player_name} gained {resource} from A good year")
            },
        )
        .build(),
    ]
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
