use crate::action_card::on_play_action_card;
use crate::advance::on_advance;
use crate::city::MoodState;
use crate::collect::on_collect;
use crate::combat::{
    combat_loop, move_with_possible_combat, on_capture_undefended_position, start_combat,
};
use crate::combat_listeners::{combat_round_end, combat_round_start, end_combat};
use crate::construct::on_construct;
use crate::content::persistent_events::{EventResponse, PersistentEventType};
use crate::cultural_influence::ask_for_cultural_influence_payment;
use crate::explore::{ask_explore_resolution, move_to_unexplored_tile};
use crate::game::GameState::{Finished, Movement, Playing};
use crate::game::{Game, GameState};
use crate::incident::on_trigger_incident;
use crate::log;
use crate::log::{add_action_log_item, current_player_turn_log_mut};
use crate::map::Terrain::Unexplored;
use crate::movement::MovementAction::{Move, Stop};
use crate::movement::{
    CurrentMove, MoveState, MovementAction, get_move_state, has_movable_units,
    move_units_destinations,
};
use crate::objective_card::{
    on_objective_cards,  present_objective_cards,
};
use crate::playing_actions::{PlayingAction, on_found_city};
use crate::recruit::on_recruit;
use crate::resource::check_for_waste;
use crate::resource_pile::ResourcePile;
use crate::status_phase::play_status_phase;
use crate::undo::{clean_patch, redo, to_serde_value, undo};
use crate::unit::{get_current_move, units_killed};
use crate::wonder::{draw_wonder_card, on_play_wonder_card};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Action {
    Playing(PlayingAction),
    Movement(MovementAction),
    Response(EventResponse),
    Undo,
    Redo,
}

impl Action {
    #[must_use]
    pub fn playing(self) -> Option<PlayingAction> {
        if let Self::Playing(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn movement(self) -> Option<MovementAction> {
        if let Self::Movement(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn response(self) -> Option<EventResponse> {
        if let Self::Response(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

///
///
/// # Panics
///
/// Panics if the action is illegal
#[must_use]
pub fn execute_action(mut game: Game, action: Action, player_index: usize) -> Game {
    let add_undo = !matches!(&action, Action::Undo);
    let old = to_serde_value(&game);
    let old_player = game.active_player();
    game = execute_without_undo(game, action, player_index).expect("action should be executed");
    let new = to_serde_value(&game);
    let new_player = game.active_player();
    let patch = json_patch::diff(&new, &old);
    if old_player != new_player {
        game.lock_undo(); // don't undo player change
    } else if add_undo && game.can_undo() {
        let i = game.action_log_index - 1;
        current_player_turn_log_mut(&mut game).items[i].undo = clean_patch(patch.0);
    }
    game
}

fn execute_without_undo(
    mut game: Game,
    action: Action,
    player_index: usize,
) -> Result<Game, String> {
    if player_index != game.active_player() {
        return Err("Not your turn".to_string());
    }
    if let Action::Undo = action {
        if !game.can_undo() {
            return Err("actions revealing new information can't be undone".to_string());
        }
        return undo(game);
    }

    if matches!(action, Action::Redo) {
        if !game.can_redo() {
            return Err("action can't be redone".to_string());
        }
        redo(&mut game, player_index)?;
        return Ok(game);
    }

    add_log_item_from_action(&mut game, &action);
    add_action_log_item(&mut game, action.clone());

    match game.current_event_handler_mut() {
        Some(s) => {
            s.response = action.response();
            let details = game.current_event().event_type.clone();
            execute_custom_phase_action(&mut game, player_index, details)
        }
        _ => execute_regular_action(&mut game, action, player_index),
    }?;
    check_for_waste(&mut game);

    if game
        .player(player_index)
        .cities
        .iter()
        .filter(|c| c.mood_state == MoodState::Angry)
        .count()
        >= 4
    {
        // todo test with regular increase mood and with persistent events
        present_objective_cards(&mut game, player_index, vec!["Terror Regime".to_string()])
    }

    Ok(game)
}

pub(crate) fn execute_custom_phase_action(
    game: &mut Game,
    player_index: usize,
    details: PersistentEventType,
) -> Result<(), String> {
    use PersistentEventType::*;
    match details {
        Collect(i) => on_collect(game, player_index, i),
        DrawWonderCard => {
            draw_wonder_card(game, player_index);
        }
        ExploreResolution(r) => {
            ask_explore_resolution(game, player_index, r);
        }
        InfluenceCultureResolution(r) => {
            ask_for_cultural_influence_payment(game, player_index, r);
        }
        UnitsKilled(k) => units_killed(game, player_index, k),
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
        CombatEnd(r) => {
            end_combat(game, r);
        }
        CaptureUndefendedPosition(c) => {
            on_capture_undefended_position(game, player_index, c);
        }
        StatusPhase(s) => play_status_phase(game, s),
        TurnStart => game.on_start_turn(),
        Advance(a) => {
            on_advance(game, player_index, a);
        }
        Construct(b) => {
            on_construct(game, player_index, b);
        }
        Recruit(r) => {
            on_recruit(game, player_index, r);
        }
        FoundCity(p) => on_found_city(game, player_index, p),
        Incident(i) => on_trigger_incident(game, i),
        ActionCard(a) => on_play_action_card(game, player_index, a),
        WonderCard(w) => on_play_wonder_card(game, player_index, w),
        SelectObjectives(c) => {
            on_objective_cards(game, player_index, c);
        }
    }

    if let Some(mut s) = game.events.pop() {
        if s.player.handler.is_none() {
            if let Some(l) = s.player.last_priority_used.as_mut() {
                *l -= 1;
            } else {
                s.player.skip_first_priority = true;
            }
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

pub(crate) fn add_log_item_from_action(game: &mut Game, action: &Action) {
    game.log.push(log::format_action_log_item(action, game));
}

fn execute_regular_action(
    game: &mut Game,
    action: Action,
    player_index: usize,
) -> Result<(), String> {
    match game.state {
        Playing => {
            if let Some(m) = action.clone().movement() {
                if let MovementAction::Move(_) = m {
                } else {
                    return Err("Expected move action".to_string());
                }
                if game.actions_left == 0 {
                    return Err("No actions left".to_string());
                }
                game.actions_left -= 1;
                game.state = GameState::Movement(MoveState::new());
                execute_movement_action(game, m, player_index)
            } else {
                let action = action.playing().expect("action should be a playing action");
                action.execute(game, player_index, false)
            }
        }
        Movement(_) => {
            let action = action
                .movement()
                .expect("action should be a movement action");
            execute_movement_action(game, action, player_index)
        }
        Finished => Err("actions can't be executed when the game is finished".to_string()),
    }
}

pub(crate) fn execute_movement_action(
    game: &mut Game,
    action: MovementAction,
    player_index: usize,
) -> Result<(), String> {
    match action {
        Move(m) => {
            let player = &game.players[player_index];
            let starting_position = player
                .get_unit(*m.units.first().expect(
                    "instead of providing no units to move a stop movement actions should be done",
                ))
                .position;
            let destinations = move_units_destinations(
                player,
                game,
                &m.units,
                starting_position,
                m.embark_carrier_id,
            )?;
            let c = &destinations
                .iter()
                .find(|route| route.destination == m.destination)
                .expect("destination should be a valid destination")
                .cost;
            if c.is_free() {
                assert_eq!(m.payment, ResourcePile::empty(), "payment should be empty");
            } else {
                game.players[player_index].pay_cost(c, &m.payment);
            }

            let current_move = get_current_move(
                game,
                &m.units,
                starting_position,
                m.destination,
                m.embark_carrier_id,
            );
            let move_state = get_move_state(game);
            move_state.moved_units.extend(m.units.iter());
            move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();

            if matches!(current_move, CurrentMove::None) || move_state.current_move != current_move
            {
                move_state.movement_actions_left -= 1;
                move_state.current_move = current_move;
            }
            if !starting_position.is_neighbor(m.destination) {
                // roads move ends the current move
                move_state.current_move = CurrentMove::None;
            }

            let dest_terrain = game
                .map
                .get(m.destination)
                .expect("destination should be a valid tile");

            if dest_terrain == &Unexplored {
                move_to_unexplored_tile(
                    game,
                    player_index,
                    &m.units,
                    starting_position,
                    m.destination,
                );
                return Ok(());
            }

            if move_with_possible_combat(game, player_index, starting_position, &m) {
                return Ok(());
            }
        }
        Stop => {
            game.state = GameState::Playing;
            return Ok(());
        }
    }

    let state = get_move_state(game);
    let all_moves_used =
        state.movement_actions_left == 0 && state.current_move == CurrentMove::None;
    if all_moves_used || !has_movable_units(game, game.player(game.current_player_index)) {
        game.state = GameState::Playing;
    }
    Ok(())
}
