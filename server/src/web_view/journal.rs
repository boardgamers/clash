use crate::game::Game;
use crate::incident::IncidentBaseEffect;
use crate::player_events::IncidentTarget;
use serde_json::{Value, json};

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
