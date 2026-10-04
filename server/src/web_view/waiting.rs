use crate::advance::Advance;
use crate::content::persistent_events::{PersistentEventRequest as Request, PersistentEventType};
use crate::events::EventOrigin;
use crate::game::{Game, GameState};
use crate::wonder::Wonder;
use serde_json::{Value, json};

// Public context only: never inspect another player's choices or name a card in hand.
pub(super) fn describe(game: &Game) -> Option<Value> {
    if game.state == GameState::Finished || game.civilization_draft.is_some() {
        return None;
    }
    let player = game.active_player();
    if let Some(handler) = game.current_event_handler() {
        if handler.response.is_some() {
            return None;
        }
        let objective = game.events.last().is_some_and(|event| {
            matches!(event.event_type, PersistentEventType::SelectObjectives(_))
        });
        let action = match (&handler.request, &handler.origin) {
            (Request::SelectUnitType(_), EventOrigin::Ability(name))
                if name == "Barbarian reinforcements" =>
            {
                "Choose a Barbarian reinforcement"
            }
            (Request::SelectPositions(_), EventOrigin::Advance(Advance::Fanaticism)) => {
                "Place an infantry"
            }
            (Request::SelectPositions(_), EventOrigin::Ability(name))
                if name == "Place Settler" =>
            {
                "Place a settler after losing a city"
            }
            (Request::SelectUnits(_), EventOrigin::Ability(name))
                if name == "Choose Casualties" =>
            {
                "Choose casualties"
            }
            (Request::BoolRequest(_), EventOrigin::Ability(name)) if name == "Retreat" => {
                "Decide whether to retreat"
            }
            (Request::SelectHandCards(_), _) if objective => "Choose an objective to complete",
            (Request::SelectHandCards(_), EventOrigin::Advance(Advance::Tactics)) => {
                "Choose a battle card"
            }
            (Request::SelectHandCards(_), EventOrigin::LeaderAbility(name))
                if name == "Loyalty" =>
            {
                "Choose a battle card"
            }
            (Request::SelectHandCards(_), _) => "Choose a card",
            (Request::Payment(_), _) => "Choose a payment",
            (Request::ResourceReward(_), _) => "Choose resources",
            (Request::SelectAdvance(_), _) => "Choose an advance",
            (Request::SelectPlayer(_), _) => "Choose a player",
            (Request::SelectPositions(_), _) => "Choose a location",
            (Request::SelectUnitType(_), _) => "Choose a unit type",
            (Request::SelectUnits(_), _) => "Choose units",
            (Request::SelectCaptives(_), _) => "Choose captives",
            (Request::SelectStructures(_), _) => "Choose a building",
            (Request::BoolRequest(_), _) => "Make a choice",
            (Request::ChangeGovernment, _) => "Choose a government",
            (Request::ExploreResolution, _) => "Place explored terrain",
        };
        let source = match &handler.origin {
            EventOrigin::Advance(_)
            | EventOrigin::SpecialAdvance(_)
            | EventOrigin::LeaderAbility(_)
            | EventOrigin::Incident(_) => Some(handler.origin.name(game)),
            EventOrigin::Wonder(wonder) if *wonder != Wonder::Hidden => {
                Some(wonder.name().to_string())
            }
            _ => None,
        };
        return Some(json!({"player":player,"action":action,"source":source}));
    }
    if !game.events.is_empty() {
        return None;
    }
    let action = match game.state {
        GameState::Movement(_) => "Finish moving",
        GameState::Playing if game.actions_left == 0 => "End their turn",
        GameState::ChooseCivilization => "Choose a civilization",
        _ => return None,
    };
    Some(json!({"player":player,"action":action,"source":null}))
}
