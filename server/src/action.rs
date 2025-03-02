use crate::advance::gain_advance;
use crate::city_pieces::Building::Temple;
use crate::combat::{
    combat_loop, combat_round_end, end_combat, move_with_possible_combat, start_combat, take_combat,
};
use crate::content::custom_phase_actions::CurrentEventResponse;
use crate::cultural_influence::execute_cultural_influence_resolution_action;
use crate::explore::{explore_resolution, move_to_unexplored_tile};
use crate::game::GameState::{
    Combat, CulturalInfluenceResolution, ExploreResolution, Finished, Movement, Playing,
    StatusPhase,
};
use crate::game::{ActionLogItem, Game};
use crate::incident::trigger_incident;
use crate::log;
use crate::map::Rotation;
use crate::map::Terrain::Unexplored;
use crate::movement::{back_to_move, move_units_destinations, CurrentMove, MoveState};
use crate::playing_actions::PlayingAction;
use crate::recruit::on_recruit;
use crate::resource::check_for_waste;
use crate::resource_pile::ResourcePile;
use crate::status_phase::StatusPhaseAction;
use crate::undo::{redo, undo, DisembarkUndoContext, UndoContext};
use crate::unit::MovementAction::{Move, Stop};
use crate::unit::{get_current_move, MovementAction};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum Action {
    Playing(PlayingAction),
    StatusPhase(StatusPhaseAction),
    Movement(MovementAction),
    CulturalInfluenceResolution(bool),
    ExploreResolution(Rotation),
    CustomPhaseEvent(CurrentEventResponse),
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
    pub fn status_phase(self) -> Option<StatusPhaseAction> {
        if let Self::StatusPhase(v) = self {
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
    pub fn cultural_influence_resolution(self) -> Option<bool> {
        if let Self::CulturalInfluenceResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn explore_resolution(self) -> Option<Rotation> {
        if let Self::ExploreResolution(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[must_use]
    pub fn custom_phase_event(self) -> Option<CurrentEventResponse> {
        if let Self::CustomPhaseEvent(v) = self {
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
pub fn execute_action(game: &mut Game, action: Action, player_index: usize) {
    assert!(player_index == game.active_player(), "Illegal action");
    if let Action::Undo = action {
        assert!(
            game.can_undo(),
            "actions revealing new information can't be undone"
        );
        undo(game, player_index);
        return;
    }

    if matches!(action, Action::Redo) {
        assert!(game.can_redo(), "no action can be redone");
        redo(game, player_index);
        return;
    }

    add_log_item_from_action(game, &action);
    add_action_log_item(game, action.clone());

    if let Some(s) = game.current_event_handler_mut() {
        s.response = action.custom_phase_event();
        let event_type = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, &event_type);
    } else {
        execute_regular_action(game, action, player_index);
    }
    check_for_waste(game);

    game.action_log[game.action_log_index - 1].undo = std::mem::take(&mut game.undo_context_stack);
}

fn add_action_log_item(game: &mut Game, item: Action) {
    if game.action_log_index < game.action_log.len() {
        game.action_log.drain(game.action_log_index..);
    }
    game.action_log.push(ActionLogItem::new(item));
    game.action_log_index += 1;
}

pub(crate) fn execute_custom_phase_action(game: &mut Game, player_index: usize, event_type: &str) {
    match event_type {
        "on_combat_start" => {
            start_combat(game);
        }
        "on_combat_round_end" => {
            game.lock_undo();
            if let Some(c) = combat_round_end(game) {
                combat_loop(game, c);
            }
        }
        "on_combat_end" => {
            let c = take_combat(game);
            end_combat(game, c);
        }
        "on_turn_start" => game.start_turn(),
        // name and payment is ignored here
        "on_advance_custom_phase" => {
            gain_advance(game, player_index, ResourcePile::empty(), "");
        }
        "on_construct" => {
            // building is ignored here
            PlayingAction::on_construct(game, player_index, Temple);
        }
        "on_recruit" => {
            on_recruit(game, player_index);
        }
        "on_incident" => {
            trigger_incident(game, player_index);
        }
        _ => panic!("unknown custom phase event {event_type}"),
    }
}

pub(crate) fn add_log_item_from_action(game: &mut Game, action: &Action) {
    game.log.push(log::format_action_log_item(action, game));
}

fn execute_regular_action(game: &mut Game, action: Action, player_index: usize) {
    match game.state.clone() {
        Playing => {
            if let Some(m) = action.clone().movement() {
                execute_movement_action(game, m, player_index, MoveState::new());
            } else {
                let action = action.playing().expect("action should be a playing action");
                action.execute(game, player_index);
            }
        }
        StatusPhase(phase) => {
            let action = action
                .status_phase()
                .expect("action should be a status phase action");
            assert!(phase == action.phase(), "Illegal action: Same phase again");
            action.execute(game, player_index);
        }
        Movement(m) => {
            let action = action
                .movement()
                .expect("action should be a movement action");
            execute_movement_action(game, action, player_index, m);
        }
        CulturalInfluenceResolution(c) => {
            let action = action
                .cultural_influence_resolution()
                .expect("action should be a cultural influence resolution action");
            execute_cultural_influence_resolution_action(
                game,
                action,
                c.roll_boost_cost,
                c.target_player_index,
                c.target_city_position,
                c.city_piece,
                player_index,
            );
        }
        Combat(_) => {
            panic!("actions can't be executed when the game is in a combat state");
        }
        ExploreResolution(r) => {
            let rotation = action
                .explore_resolution()
                .expect("action should be an explore resolution action");
            explore_resolution(game, &r, rotation);
        }
        Finished => panic!("actions can't be executed when the game is finished"),
    }
}

pub(crate) fn execute_movement_action(
    game: &mut Game,
    action: MovementAction,
    player_index: usize,
    mut move_state: MoveState,
) {
    let saved_state = move_state.clone();
    let (starting_position, disembarked_units) = match action {
        Move(m) => {
            if let Playing = game.state {
                assert_ne!(game.actions_left, 0, "Illegal action");
                game.actions_left -= 1;
            }
            let player = &game.players[player_index];
            let starting_position = player
                .get_unit(*m.units.first().expect(
                    "instead of providing no units to move a stop movement actions should be done",
                ))
                .expect("the player should have all units to move")
                .position;
            let disembarked_units = m
                .units
                .iter()
                .filter_map(|unit| {
                    let unit = player.get_unit(*unit).expect("unit should exist");
                    unit.carrier_id.map(|carrier_id| DisembarkUndoContext {
                        unit_id: unit.id,
                        carrier_id,
                    })
                })
                .collect();
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

            let dest_terrain = game
                .map
                .get(m.destination)
                .expect("destination should be a valid tile");

            if dest_terrain == &Unexplored {
                if move_to_unexplored_tile(
                    game,
                    player_index,
                    &m.units,
                    starting_position,
                    m.destination,
                    &move_state,
                ) {
                    back_to_move(game, &move_state, true);
                }
                return;
            }

            if move_with_possible_combat(
                game,
                player_index,
                Some(&mut move_state),
                starting_position,
                &m,
            ) {
                return;
            }

            (Some(starting_position), disembarked_units)
        }
        Stop => {
            game.state = Playing;
            (None, Vec::new())
        }
    };
    game.push_undo_context(UndoContext::Movement {
        starting_position,
        move_state: saved_state,
        disembarked_units,
    });
}
