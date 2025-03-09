use crate::advance::gain_advance;
use crate::combat::{
    combat_loop, combat_round_end, end_combat, move_with_possible_combat, start_combat, take_combat,
};
use crate::content::custom_phase_actions::{CurrentEventResponse, CurrentEventType};
use crate::cultural_influence::ask_for_cultural_influence_payment;
use crate::explore::{ask_explore_resolution, move_to_unexplored_tile};
use crate::game::GameState::{Combat, Finished, Movement, Playing, StatusPhase};
use crate::game::{ActionLogItem, Game, GameState};
use crate::incident::trigger_incident;
use crate::log;
use crate::map::Terrain::Unexplored;
use crate::movement::{
    has_movable_units, move_units_destinations, stop_current_move, take_move_state, CurrentMove,
    MoveState,
};
use crate::playing_actions::PlayingAction;
use crate::recruit::on_recruit;
use crate::resource::check_for_waste;
use crate::resource_pile::ResourcePile;
use crate::status_phase::play_status_phase;
use crate::undo::{clean_patch, redo, to_serde_value, undo};
use crate::unit::MovementAction::{Move, Stop};
use crate::unit::{get_current_move, MovementAction};
use crate::wonder::draw_wonder_card;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Action {
    Playing(PlayingAction),
    Movement(MovementAction),
    Response(CurrentEventResponse),
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
    pub fn playing_ref(&self) -> Option<&PlayingAction> {
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
    pub fn custom_phase_event(self) -> Option<CurrentEventResponse> {
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
    game = execute_without_undo(game, action, player_index);
    let new = to_serde_value(&game);
    let new_player = game.active_player();
    let patch = json_patch::diff(&new, &old);
    if old_player != new_player {
        game.lock_undo();
    } else if add_undo && game.can_undo() {
        game.action_log[game.action_log_index - 1].undo = clean_patch(patch.0);
    }
    game
}

fn execute_without_undo(mut game: Game, action: Action, player_index: usize) -> Game {
    assert!(player_index == game.active_player(), "Illegal action");
    if let Action::Undo = action {
        assert!(
            game.can_undo(),
            "actions revealing new information can't be undone"
        );
        return undo(game);
    }

    if matches!(action, Action::Redo) {
        assert!(game.can_redo(), "action can't be redone");
        redo(&mut game, player_index);
        return game;
    }

    add_log_item_from_action(&mut game, &action);
    add_action_log_item(&mut game, action.clone());

    if let Some(s) = game.current_event_handler_mut() {
        s.response = action.custom_phase_event();
        let details = game.current_event().event_type.clone();
        execute_custom_phase_action(&mut game, player_index, &details);
    } else {
        execute_regular_action(&mut game, action, player_index);
    }
    check_for_waste(&mut game);
    game
}

fn add_action_log_item(game: &mut Game, item: Action) {
    if game.action_log_index < game.action_log.len() {
        game.action_log.drain(game.action_log_index..);
    }
    game.action_log.push(ActionLogItem::new(item));
    game.action_log_index += 1;
}

pub(crate) fn execute_custom_phase_action(
    game: &mut Game,
    player_index: usize,
    details: &CurrentEventType,
) {
    use CurrentEventType::*;
    match details {
        DrawWonderCard => {
            draw_wonder_card(game, player_index);
        }
        ExploreResolution(r) => {
            ask_explore_resolution(game, player_index, r);
        }
        InfluenceCultureResolution(r) => {
            ask_for_cultural_influence_payment(game, player_index, r);
        }
        CombatStart => {
            start_combat(game);
        }
        CombatRoundEnd(r) => {
            game.lock_undo();
            if let Some(c) = combat_round_end(game, r) {
                combat_loop(game, c);
            }
        }
        CombatEnd(r) => {
            let c = take_combat(game);
            end_combat(game, c, r.clone());
        }
        StatusPhase => play_status_phase(game),
        TurnStart => game.start_turn(),
        Advance(a) => {
            gain_advance(game, player_index, a);
        }
        Construct(b) => {
            PlayingAction::on_construct(game, player_index, *b);
        }
        Recruit(r) => {
            on_recruit(game, player_index, r);
        }
        Incident(i) => {
            trigger_incident(game, player_index, i);
        }
    }
}

pub(crate) fn add_log_item_from_action(game: &mut Game, action: &Action) {
    game.log.push(log::format_action_log_item(action, game));
}

fn execute_regular_action(game: &mut Game, action: Action, player_index: usize) {
    match game.state() {
        Playing => {
            if let Some(m) = action.clone().movement() {
                assert_ne!(game.actions_left, 0, "Illegal action");
                game.actions_left -= 1;
                game.push_state(GameState::Movement(MoveState::new()));
                execute_movement_action(game, m, player_index);
            } else {
                let action = action.playing().expect("action should be a playing action");
                action.execute(game, player_index);
            }
        }
        Movement(_) => {
            let action = action
                .movement()
                .expect("action should be a movement action");
            execute_movement_action(game, action, player_index);
        }
        Combat(_) => {
            panic!("actions can't be executed when the game is in a combat state");
        }
        StatusPhase(_) => {
            panic!("actions can't be executed when the game is in a status state");
        }
        Finished => panic!("actions can't be executed when the game is finished"),
    }
}

pub(crate) fn execute_movement_action(
    game: &mut Game,
    action: MovementAction,
    player_index: usize,
) {
    match action {
        Move(m) => {
            let player = &game.players[player_index];
            let starting_position = player
                .get_unit(*m.units.first().expect(
                    "instead of providing no units to move a stop movement actions should be done",
                ))
                .position;
            match move_units_destinations(
                player,
                game,
                &m.units,
                starting_position,
                m.embark_carrier_id,
            ) {
                Ok(destinations) => {
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
                }
                Err(e) => {
                    panic!("cannot move units to destination: {e}");
                }
            }

            let mut move_state = take_move_state(game);
            move_state.moved_units.extend(m.units.iter());
            move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();
            let current_move = get_current_move(
                game,
                &m.units,
                starting_position,
                m.destination,
                m.embark_carrier_id,
            );

            if matches!(current_move, CurrentMove::None) || move_state.current_move != current_move
            {
                move_state.movement_actions_left -= 1;
                move_state.current_move = current_move;
            }
            if !starting_position.is_neighbor(m.destination) {
                // roads move ends the current move
                move_state.current_move = CurrentMove::None;
            }
            game.push_state(GameState::Movement(move_state));

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
                stop_current_move(game);
                return; // can't undo this action
            }

            if move_with_possible_combat(game, player_index, starting_position, &m) {
                return;
            }
        }
        Stop => {
            game.pop_state();
            return;
        }
    };

    let state = take_move_state(game);
    let all_moves_used =
        state.movement_actions_left == 0 && state.current_move == CurrentMove::None;
    if all_moves_used || !has_movable_units(game, game.get_player(game.current_player_index)) {
        return;
    }
    game.push_state(GameState::Movement(state));
}
