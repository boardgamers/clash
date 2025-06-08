use crate::action_card::on_play_action_card;
use crate::advance::on_advance;
use crate::barbarians::on_stop_barbarian_movement;
use crate::city::{MoodState, on_found_city};
use crate::collect::on_collect;
use crate::combat::{combat_loop, start_combat};
use crate::combat_listeners::{combat_round_end, combat_round_start, on_end_combat};
use crate::construct::on_construct;
use crate::content::custom_actions::on_custom_action;
use crate::content::persistent_events::{EventResponse, PersistentEventType};
use crate::cultural_influence::on_cultural_influence;
use crate::explore::ask_explore_resolution;
use crate::game::GameState::{Finished, Movement, Playing};
use crate::game::{Game, GameContext};
use crate::incident::{on_choose_incident, on_trigger_incident};
use crate::log::{add_action_log_item, current_player_turn_log_mut};
use crate::movement::{MovementAction, execute_movement_action, on_ship_construction_conversion};
use crate::objective_card::{complete_objective_card, gain_objective_card, on_objective_cards};
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::position::Position;
use crate::recruit::on_recruit;
use crate::resource::check_for_waste;
use crate::status_phase::play_status_phase;
use crate::undo::{clean_patch, redo, to_serde_value, undo};
use crate::unit::units_killed;
use crate::wonder::{on_draw_wonder_card, on_play_wonder_card};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Action {
    Playing(PlayingAction),
    Movement(MovementAction),
    Response(EventResponse),
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionType {
    Playing(PlayingActionType),
    Movement,
    Response,
    Undo,
    Redo,
}

///
///
/// # Panics
///
/// Panics if the action is illegal
#[must_use]
pub fn execute_action(game: Game, action: Action, player_index: usize) -> Game {
    try_execute_action(game, action, player_index).expect("could not execute action")
}

///
///
/// # Errors
///
/// Returns an error if the action is not valid
pub fn try_execute_action(
    mut game: Game,
    action: Action,
    player_index: usize,
) -> Result<Game, String> {
    if player_index != game.active_player() {
        return Err(format!("Player {player_index} is not the active player"));
    }

    if game.context == GameContext::AI {
        return execute_without_undo(&mut game, action, player_index).map(|()| game);
    }

    if let Action::Undo = action {
        if !game.can_undo() {
            return Err("action can't be undone".to_string());
        }
        return undo(game);
    }

    let add_undo = !matches!(&action, Action::Undo);
    let old = to_serde_value(&game);
    let old_player = game.active_player();
    execute_without_undo(&mut game, action, player_index)?;
    let new = to_serde_value(&game);
    let new_player = game.active_player();
    let patch = json_patch::diff(&new, &old);
    if old_player != new_player {
        game.player_changed();
    } else if add_undo && game.can_undo() {
        let i = game.action_log_index - 1;
        current_player_turn_log_mut(&mut game).items[i].undo = clean_patch(patch.0);
    }
    Ok(game)
}

///
/// # Errors
///
/// Returns an error if the action is not valid
pub fn execute_without_undo(
    game: &mut Game,
    action: Action,
    player_index: usize,
) -> Result<(), String> {
    game.log.push(vec![]);
    if matches!(action, Action::Redo) {
        if !game.can_redo() {
            return Err("action can't be redone".to_string());
        }
        redo(game, player_index)?;
        return Ok(());
    }

    add_action_log_item(game, action.clone());

    if game.context == GameContext::Replay
        && !game.events.is_empty()
        && !matches!(action, Action::Response(_))
    {
        // ignore missing response in replay
        game.add_info_log_item(&format!(
            "interrupted {} events in replay due to a missing response",
            game.events.len()
        ));
        game.events.clear();
    }

    match game.current_event_handler_mut() {
        Some(s) => {
            s.response = if let Action::Response(v) = action {
                Some(v)
            } else {
                return Err(format!(
                    "action should be a response: {action:?} - event: {s:?}"
                ));
            };
            let details = game.current_event().event_type.clone();
            execute_custom_phase_action(game, player_index, details)
        }
        _ => execute_regular_action(game, action, player_index),
    }?;

    after_action(game, player_index);
    Ok(())
}

pub(crate) fn after_action(game: &mut Game, player_index: usize) {
    check_for_waste(game);

    if let Some(o) = game.player_mut(player_index).gained_objective.take() {
        gain_objective_card(game, player_index, o);
    }

    if game
        .player(player_index)
        .cities
        .iter()
        .filter(|c| c.mood_state == MoodState::Angry)
        .count()
        >= 4
        && !current_player_turn_log_mut(game).items.is_empty()
    {
        //endless loop if this is not selected automatically
        let card = game
            .player(player_index)
            .objective_cards
            .iter()
            .find(|o| **o == 29);
        if let Some(o) = card {
            // actually did an action
            complete_objective_card(game, player_index, *o, "Terror Regime".to_string());
        }
    }

    let p = game.player_mut(player_index);
    if p.great_mausoleum_action_cards > 0 {
        p.great_mausoleum_action_cards -= 1;
        on_action_end(game, player_index);
    }
    let mut city: Option<Position> = None;
    for c in &mut game.player_mut(player_index).cities {
        if c.activation_mood_decreased {
            city = Some(c.position);
        }
        c.activation_mood_decreased = false;
    }
    if let Some(pos) = city {
        on_city_activation_mood_decreased(game, player_index, pos);
    }
}

pub(crate) fn on_city_activation_mood_decreased(
    game: &mut Game,
    player_index: usize,
    pos: Position,
) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.city_activation_mood_decreased,
        pos,
        PersistentEventType::CityActivationMoodDecreased,
    );
}

pub(crate) fn on_action_end(game: &mut Game, player_index: usize) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.choose_action_card,
        (),
        |()| PersistentEventType::ChooseActionCard,
    );
}

pub(crate) fn execute_custom_phase_action(
    game: &mut Game,
    player: usize,
    details: PersistentEventType,
) -> Result<(), String> {
    use PersistentEventType::*;
    match details {
        Collect(i) => on_collect(game, player, i),
        DrawWonderCard(drawn) => {
            on_draw_wonder_card(game, player, drawn);
        }
        ExploreResolution(r) => {
            ask_explore_resolution(game, player, r);
        }
        InfluenceCulture(r) => {
            on_cultural_influence(game, player, r);
        }
        UnitsKilled(k) => units_killed(game, player, k),
        CombatStart(c) => {
            start_combat(game, c);
        }
        CombatRoundStart(r) => {
            if let Some(c) = combat_round_start(game, r) {
                combat_loop(game, c);
            }
        }
        CombatRoundEnd(r) => {
            if let Some(c) = combat_round_end(game, r) {
                combat_loop(game, crate::combat_listeners::CombatRoundStart::new(c));
            }
        }
        CombatEnd(s) => {
            on_end_combat(game, s);
        }
        StatusPhase(s) => play_status_phase(game, s),
        TurnStart => game.on_start_turn(),
        PayAction(a) => {
            a.on_pay_action(game, player, game.current_event().origin_override.clone())?;
        }
        Advance(a) => {
            on_advance(game, player, a);
        }
        Construct(b) => {
            on_construct(game, player, b);
        }
        Recruit(r) => {
            on_recruit(game, player, r);
        }
        FoundCity(p) => on_found_city(game, player, p),
        Incident(i) => on_trigger_incident(game, i),
        StopBarbarianMovement(movable) => on_stop_barbarian_movement(game, movable),
        ActionCard(a) => on_play_action_card(game, player, a),
        WonderCard(w) => on_play_wonder_card(game, player, w),
        SelectObjectives(c) => {
            on_objective_cards(game, player, c);
        }
        CustomAction(a) => on_custom_action(game, player, a),
        ChooseActionCard => on_action_end(game, player),
        ChooseIncident(i) => on_choose_incident(game, player, i),
        CityActivationMoodDecreased(p) => {
            on_city_activation_mood_decreased(game, player, p);
        }
        ShipConstructionConversion(u) => on_ship_construction_conversion(game, player, u),
    }

    if let Some(s) = game.events.pop() {
        if s.player.handler.is_none() {
            let p = s.player.index;
            let event_type = s.event_type.clone();
            game.events.push(s);
            execute_custom_phase_action(game, p, event_type)?;
        } else {
            game.events.push(s);
        }
    }
    Ok(())
}

fn execute_regular_action(
    game: &mut Game,
    action: Action,
    player_index: usize,
) -> Result<(), String> {
    match game.state {
        Playing => {
            if let Action::Movement(m) = action {
                execute_movement_action(game, m.clone(), player_index)
            } else {
                let Action::Playing(action) = action else {
                    return Err(format!("Action {action:?} is not a playing action"));
                };
                action.execute(game, player_index, false)
            }
        }
        Movement(_) => execute_movement_action(
            game,
            if let Action::Movement(v) = action {
                v
            } else {
                return Err(format!("action {action:?} is not a movement action"));
            },
            player_index,
        ),
        Finished => Err("actions can't be executed when the game is finished".to_string()),
    }
}
