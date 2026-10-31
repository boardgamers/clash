//! Private plans around narrowly identified off-turn reactions. The real event
//! stays in place until its owner responds; plans execute only afterwards.
use crate::{
    action::{Action, try_execute_action},
    content::persistent_events::{PersistentEventRequest, PersistentEventType},
    events::EventOrigin,
    game::Game,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub player: usize,
    pub key: String,
    pub actions: Vec<Action>,
    pub waiting: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn offer(game: &Game, player: usize) -> Option<(String, String)> {
    if player != game.current_player_index
        || player == game.active_player()
        || game.round > 3
        || game.events.len() != 1
        || game.civilization_draft.is_some()
    {
        return None;
    }
    let event = game.events.last()?;
    let handler = event.player.handler.as_ref()?;
    let label = match (&event.event_type, &handler.origin, &handler.request) {
        (
            PersistentEventType::CombatEnd(_),
            EventOrigin::Ability(name),
            PersistentEventRequest::SelectPositions(_),
        ) if name == "Place Settler" => "Placing a settler",
        (PersistentEventType::ActionCard(card), _, PersistentEventRequest::SelectAdvance(_))
            if card.active_player == Some(player)
                && card.selected_player == Some(game.active_player())
                && game.cache.get_action_card(card.id).civil_card.name == "Technology Trade" =>
        {
            "Choosing an advance"
        }
        (PersistentEventType::Incident(i), EventOrigin::Incident(id),
            PersistentEventRequest::SelectPositions(request))
            if i.active_player == player && *id == i.incident_id
                && [27, 28, 34, 35].contains(id) && handler.priority == 11
                && i.selected_player == Some(game.active_player())
                && request.choices.iter().all(|position|
                    game.player(game.active_player()).cities.iter().any(|city| city.position == *position)) =>
        {
            "Placing a settler"
        }
        (PersistentEventType::Incident(i), EventOrigin::Incident(id),
            PersistentEventRequest::Payment(requests))
            if i.active_player == player && *id == i.incident_id
                && !i.consumed && !i.replaced_by_sacrifice && i.selected_player.is_none()
                && game.cache.get_incident(*id).action_card.is_some()
                && handler.priority == 10
                && requests.iter().any(|request| request.name == "Pay to gain the Action Card") =>
        {
            game.cache.get_incident(*id).name.as_str()
        }
        (PersistentEventType::Incident(i), EventOrigin::Incident(id),
            PersistentEventRequest::SelectPlayer(_))
            if i.active_player == player && *id == 57 && i.incident_id == *id
                && handler.priority == 1 && i.selected_player == Some(game.active_player()) =>
        {
            "Choose a player"
        }
        _ => return None,
    };
    // Passing a Great Person to another recipient (or choosing a Diplomat
    // partner) continues the same unresolved reaction. Preserve queued actions
    // and the explicit Wait preference while that reaction progresses.
    if let Some(plan) = game.action_plan.as_ref().filter(|plan|
        plan.player == player && plan.error.is_none()) {
        return Some((plan.key.clone(), label.into()));
    }
    let key = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                game.age,
                game.round,
                crate::game_api::log_length(game),
                &game.events
            ))
            .ok()?
        )
    );
    Some((key, label.into()))
}

fn safe_step(game: Game, action: Action, player: usize) -> Result<Game, String> {
    if !matches!(
        action,
        Action::Playing(_) | Action::Movement(_) | Action::Response(_)
    ) {
        return Err("This action cannot be queued".into());
    }
    // Spy can reveal a hand without consuming randomness or a draw pile.
    if game.events.iter().any(|event| match &event.event_type {
        PersistentEventType::ActionCard(card) => [7, 8].contains(&card.id),
        PersistentEventType::PayAction(pay) => serde_json::to_value(pay)
            .ok()
            .is_some_and(|v| matches!(v["action"]["ActionCard"].as_u64(), Some(7 | 8))),
        _ => false,
    }) {
        return Err("Wait for the earlier choice before revealing a hand".into());
    }
    // A battle must not be sampled, even if its first step only asks for tactics.
    if let Action::Movement(crate::movement::MovementAction::Move(ref movement)) = action {
        if game.players.iter().any(|p| {
            p.index != player
                && p.units.iter().any(|u| {
                    u.position == movement.destination
                        && u.unit_type != crate::unit::UnitType::Settler
                })
        }) {
            return Err("Wait for the earlier choice before attacking".into());
        }
    }
    let before = serde_json::to_value(game.cloned_data()).map_err(|e| e.to_string())?;
    let result = try_execute_action(game, action, player)?;
    let after = serde_json::to_value(result.cloned_data()).map_err(|e| e.to_string())?;
    if before["rng"] != after["rng"]
        || before["dice_roll_log"] != after["dice_roll_log"]
        || [
            "incidents_left",
            "wonders_left",
            "action_cards_left",
            "objective_cards_left",
        ]
        .iter()
        .any(|key| {
            after[*key].as_array().map_or(0, Vec::len) < before[*key].as_array().map_or(0, Vec::len)
        })
        || after["map"]["unexplored_blocks"]
            .as_array()
            .map_or(0, Vec::len)
            < before["map"]["unexplored_blocks"]
                .as_array()
                .map_or(0, Vec::len)
        || result.active_player() != player
    {
        return Err(
            "Wait for the earlier choice: this action reveals information or needs another player"
                .into(),
        );
    }
    Ok(result)
}

pub fn project(game: &Game, player: usize, actions: &[Action]) -> Result<Game, String> {
    if offer(game, player).is_none() {
        return Err("Planning is no longer available".into());
    }
    let mut projected = game.clone();
    projected.events.clear();
    projected.action_plan = None;
    for (index, action) in actions.iter().enumerate() {
        if matches!(
            action,
            Action::Playing(crate::playing_actions::PlayingAction::EndTurn)
        ) {
            if index + 1 != actions.len() || !projected.events.is_empty() {
                return Err("End turn must be last, after your choices".into());
            }
            break;
        }
        projected = safe_step(projected, action.clone(), player)?;
    }
    Ok(projected)
}

pub fn save(
    game: &mut Game,
    player: usize,
    key: &str,
    actions: Vec<Action>,
    waiting: bool,
) -> Result<(), String> {
    if actions.is_empty()
        && game
            .action_plan
            .as_ref()
            .is_some_and(|p| p.player == player && p.key == key && p.error.is_some())
    {
        game.action_plan = None;
        game.early_choice_update = true;
        return Ok(());
    }
    if actions.len() > 100 {
        return Err("Too many queued actions".into());
    }
    if !offer(game, player).is_some_and(|(current, _)| current == key) {
        return Err("The earlier choice has changed; review your plan".into());
    }
    project(game, player, &actions)?;
    game.early_choice_update = true;
    game.action_plan = Some(Plan {
        player,
        key: key.into(),
        actions,
        waiting,
        error: None,
    });
    Ok(())
}

/// Called after ordinary resolution. Keep the failed suffix private for review.
pub fn resume(mut game: Game) -> Game {
    let Some(mut plan) = game.action_plan.take() else {
        return game;
    };
    if plan.error.is_some() || !game.events.is_empty() || game.active_player() != plan.player {
        game.action_plan = Some(plan);
        return game;
    }
    while !plan.actions.is_empty() {
        let action = plan.actions[0].clone();
        if !matches!(
            action,
            Action::Playing(crate::playing_actions::PlayingAction::EndTurn)
        ) {
            if let Err(error) = safe_step(game.clone(), action.clone(), plan.player) {
                plan.error = Some(error);
                game.action_plan = Some(plan);
                return game;
            }
        }
        match crate::board_history::execute(game.clone(), action, plan.player) {
            Ok(next) => {
                game = next;
                plan.actions.remove(0);
            }
            Err(error) => {
                plan.error = Some(error);
                game.action_plan = Some(plan);
                return game;
            }
        }
    }
    game
}
