use crate::action_card::on_play_action_card;
use crate::advance::on_advance;
use crate::city::{MoodState, on_found_city};
use crate::collect::on_collect;
use crate::combat::{combat_loop, move_with_possible_combat, start_combat};
use crate::combat_listeners::{combat_round_end, combat_round_start, on_end_combat};
use crate::construct::on_construct;
use crate::content::custom_actions::execute_custom_action;
use crate::content::persistent_events::{EventResponse, PersistentEventType};
use crate::cultural_influence::on_cultural_influence;
use crate::explore::{ask_explore_resolution, move_to_unexplored_tile};
use crate::game::GameState::{Finished, Movement, Playing};
use crate::game::{Game, GameContext, GameState};
use crate::incident::{on_choose_incident, on_trigger_incident};
use crate::log;
use crate::log::{add_action_log_item, current_player_turn_log_mut};
use crate::map::Terrain::Unexplored;
use crate::movement::MovementAction::{Move, Stop};
use crate::movement::{
    CurrentMove, MoveState, MoveUnits, MovementAction, has_movable_units, move_units_destinations,
};
use crate::objective_card::{complete_objective_card, gain_objective_card, on_objective_cards};
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::recruit::on_recruit;
use crate::resource::check_for_waste;
use crate::resource_pile::ResourcePile;
use crate::status_phase::play_status_phase;
use crate::undo::{clean_patch, redo, to_serde_value, undo};
use crate::unit::{get_current_move, units_killed};
use crate::wonder::{draw_wonder_card, on_play_wonder_card};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum Action {
    Playing(PlayingAction),
    Movement(MovementAction),
    Response(EventResponse),
    Undo,
    Redo,
}

impl Action {
    #[must_use]
    pub fn get_type(&self) -> ActionType {
        match self {
            Self::Playing(v) => ActionType::Playing(v.playing_action_type()),
            Self::Movement(_) => ActionType::Movement,
            Self::Response(_) => ActionType::Response,
            Self::Undo => ActionType::Undo,
            Self::Redo => ActionType::Redo,
        }
    }
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
pub fn execute_action(mut game: Game, action: Action, player_index: usize) -> Game {
    assert_eq!(player_index, game.active_player(), "Not your turn");

    if game.context == GameContext::AI {
        execute_without_undo(&mut game, action, player_index).expect("action should be executed");
        return game;
    }

    if let Action::Undo = action {
        assert!(
            game.can_undo(),
            "actions revealing new information can't be undone"
        );
        return undo(game).expect("cannot undo");
    }

    let add_undo = !matches!(&action, Action::Undo);
    let old = to_serde_value(&game);
    let old_player = game.active_player();
    execute_without_undo(&mut game, action, player_index).expect("action should be executed");
    let new = to_serde_value(&game);
    let new_player = game.active_player();
    let patch = json_patch::diff(&new, &old);
    if old_player != new_player {
        game.player_changed();
    } else if add_undo && game.can_undo() {
        let i = game.action_log_index - 1;
        current_player_turn_log_mut(&mut game).items[i].undo = clean_patch(patch.0);
    }
    game
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
    if matches!(action, Action::Redo) {
        if !game.can_redo() {
            return Err("action can't be redone".to_string());
        }
        redo(game, player_index)?;
        return Ok(());
    }

    add_log_item_from_action(game, &action);
    add_action_log_item(game, action.clone());

    match game.current_event_handler_mut() {
        Some(s) => {
            s.response = if let Action::Response(v) = action {
                Some(v)
            } else {
                return Err(format!("action should be a response: {action:?}"));
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
        InfluenceCulture(r) => {
            on_cultural_influence(game, player_index, r);
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
        CombatEnd(s) => {
            on_end_combat(game, s);
        }
        StatusPhase(s) => play_status_phase(game, s),
        TurnStart => game.on_start_turn(),
        PayAction(a) => a.on_pay_action(game, player_index)?,
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
        CustomAction(a) => execute_custom_action(game, player_index, a),
        ChooseActionCard => on_action_end(game, player_index),
        ChooseIncident(i) => on_choose_incident(game, player_index, i),
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

pub(crate) fn execute_movement_action(
    game: &mut Game,
    action: MovementAction,
    player_index: usize,
) -> Result<(), String> {
    if let GameState::Playing = game.state {
        if game.actions_left == 0 {
            return Err("No actions left".to_string());
        }
        game.actions_left -= 1;
        game.state = GameState::Movement(MoveState::new());
    }

    match action {
        Move(m) => {
            execute_move_action(game, player_index, &m)?;

            if let Movement(state) = &game.state {
                let all_moves_used =
                    state.movement_actions_left == 0 && state.current_move == CurrentMove::None;
                if all_moves_used
                    || !has_movable_units(game, game.player(game.current_player_index))
                {
                    game.state = GameState::Playing;
                }
            }
        }
        Stop => {
            game.state = GameState::Playing;
        }
    }

    Ok(())
}

fn execute_move_action(game: &mut Game, player_index: usize, m: &MoveUnits) -> Result<(), String> {
    let player = &game.players[player_index];
    let starting_position =
        player
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

    let (dest, result) = destinations
        .into_iter()
        .find(|(route, _)| route.destination == m.destination)
        .expect("destination should be a valid destination");
    result?;

    let c = &dest.cost;
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
    let Movement(move_state) = &mut game.state else {
        return Err("no move state".to_string());
    };
    move_state.moved_units.extend(m.units.iter());
    move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();

    if matches!(current_move, CurrentMove::None) || move_state.current_move != current_move {
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
    } else {
        move_with_possible_combat(game, player_index, m);
    }

    Ok(())
}
