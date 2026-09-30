use crate::game::Game;
use crate::incident::IncidentBaseEffect;
use crate::player_events::IncidentTarget;
use serde_json::{Value, json};

pub(super) fn decision_context(game: &Game, seat: usize) -> Option<Value> {
    use crate::content::persistent_events::PersistentEventType;
    let info = game.events.iter().rev().find_map(|event| {
        if let PersistentEventType::Incident(info) = &event.event_type {
            Some(info)
        } else {
            None
        }
    })?;
    let incident = game.cache.get_incident(info.incident_id);
    let handler = game.current_event_handler()?;
    let during_base = handler.priority >= crate::incident::BASE_EFFECT_PRIORITY;
    let pirate_raid = matches!(
        incident.base_effect,
        IncidentBaseEffect::PiratesSpawnAndRaid
    );
    let card = incident
        .action_card
        .as_ref()
        .filter(|_| info.selected_player.is_none())
        .map(|card| {
            let description = card
                .civil_card
                .description
                .strip_prefix(crate::content::incidents::great_persons::GREAT_PERSON_DESCRIPTION)
                .unwrap_or(&card.civil_card.description)
                .trim();
            json!({"name":card.civil_card.name,"description":description,
            "cost":{"culture_tokens":if info.active_player == seat {1} else {2}},
            "free":card.civil_card.action_type.free,"later":during_base,
            "firstOffer":info.active_player == seat})
        });
    Some(
        json!({"name":incident.name,"rules":incident.description(game),"card":card,
        "raid":if during_base && pirate_raid {Some("Each player with a city next to pirates pays 1 resource or token. If unable to pay, lower the mood of one such city.")} else {None},
        "placement":if pirate_raid {Some("Place 2 pirate ships on sea spaces without player units. Place the first next to one of your cities if possible.")} else {None}}),
    )
}

pub(super) fn catalog(game: &Game) -> Vec<Value> {
    game.cache.get_incidents().iter().map(|incident| json!({
        "id": incident.id,
        "name": incident.name,
        "rules": incident.description(game),
        "baseEffect": (!matches!(incident.base_effect, IncidentBaseEffect::None))
            .then(|| incident.base_effect.to_string()),
        "protectionAdvance": incident.protection_advance.map(|a| a.id()),
        "protectionSpecialAdvance": incident.protection_special_advance.map(|a| format!("{a:?}")),
        "minimumCities": match incident.id {
            1 | 49 => Some(2),
            29 => Some(4),
            30 | 31 => Some(3),
            _ => None,
        },
        "targets": incident.targets.iter().map(|target| match target {
            IncidentTarget::ActivePlayer => "active",
            IncidentTarget::SelectedPlayer => "selected",
            IncidentTarget::AllPlayers => "all",
        }).collect::<Vec<_>>(),
    })).collect()
}
