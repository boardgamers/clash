use crate::common::{GamePath, to_json, write_result};
use itertools::Itertools;
use server::action::ActionType;
use server::ai_actions::AiActions;
use server::game::Game;
use server::movement::{MoveUnits, MovementAction, move_units_destinations};
use server::playing_actions::PlayingActionType;
use server::profiling::start_profiling;
use server::{
    action::{self, Action},
    ai_actions,
    game::GameState,
    game_setup,
    playing_actions::PlayingAction,
    utils::{Rng, Shuffle},
};

mod common;

const ITERATIONS: usize = 100;

//is too slow if not in release mode
#[test]
fn test_random_actions() {
    start_profiling();

    let mut rng = Rng::new();
    let mut iterations = 0;
    loop {
        rng.seed = rng.seed.wrapping_add(1);
        rng.next_seed();
            let thread_rng = rng.clone();
            let handle = task::spawn(async move { random_actions_iteration(thread_rng) });
            handles.push(handle);
        }
        for handle in handles {
            handle.await;
        }
        iterations += num_cores;
        if iterations >= ITERATIONS {
            break;
        }
    }
}

fn random_actions_iteration(mut rng: Rng) {
    let seed = rng.range(0, 10_usize.pow(15)).to_string();
    let mut game = game_setup::setup_game(2, seed, true);
    game.ai_mode = true;
    let mut ai_actions = AiActions::new();
    loop {
        if matches!(game.state, GameState::Finished) {
            break;
        }
        let player_index = game.active_player();
        let mut actions = ai_actions.get_available_actions(&game);
        actions.extend(get_movement_actions(&mut ai_actions, &game));

        let actions = actions
            .into_iter()
            .flat_map(|(_, actions)| actions)
            .collect::<Vec<Action>>();
        let action = actions
            .take_random_element(&mut rng)
            .unwrap_or_else(|| no_action_available(&game));

        match action::execute_without_undo(&mut game, action.clone(), player_index) {
            Ok(_) => {}
            Err(e) => {
                use chrono::Utc;
                let rfc_format = Utc::now().to_rfc3339();
                let file = format!("failure{rfc_format}");

                write_result(&to_json(&game), &GamePath::new(".", &file));

                panic!(
                    "player {player_index} action {action:?}\nresult stored in {file}.json: {e:?}"
                )
            }
        }
    }
}

fn no_action_available(game: &Game) -> Action {
    if matches!(game.state, GameState::Movement(_)) {
        return Action::Movement(MovementAction::Stop);
    }
    Action::Playing(PlayingAction::EndTurn)
}

pub fn get_movement_actions(
    ai_actions: &mut AiActions,
    game: &Game,
) -> Vec<(ActionType, Vec<Action>)> {
    if PlayingActionType::MoveUnits
        .is_available(game, game.current_player_index)
        .is_err()
    {
        return vec![];
    }

    let p = game.player(game.current_player_index);

    // always move entire stacks as a simplification
    let actions = p
        .units
        .iter()
        .chunk_by(|u| u.position)
        .into_iter()
        .flat_map(|(pos, units)| {
            let unit_ids = units.into_iter().map(|u| u.id).collect_vec();
            let destinations = move_units_destinations(p, game, &unit_ids, pos, None);
            destinations
                .map(|d| {
                    d.iter()
                        .filter_map(|route| {
                            ai_actions::try_payment(ai_actions, &route.cost, p).map({
                                let value = unit_ids.clone();
                                move |pay| {
                                    Action::Movement(MovementAction::Move(MoveUnits::new(
                                        value,
                                        route.destination,
                                        None,
                                        pay,
                                    )))
                                }
                            })
                        })
                        .collect_vec()
                })
                .unwrap_or_default()
        })
        .collect_vec();

    if actions.is_empty() {
        return vec![];
    }
    vec![(ActionType::Playing(PlayingActionType::MoveUnits), actions)]

    // todo embark
    // || can_embark(game, p, unit)
}
