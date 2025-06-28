use crate::common::{GamePath, to_json, write_result};
use async_std::task;
use itertools::Itertools;
use server::action::ActionType;
use server::ai_actions::AiActions;
use server::cache::Cache;
use server::game::{Game, GameContext};
use server::game_setup::GameSetupBuilder;
use server::movement::{MoveUnits, MovementAction, possible_move_routes};
use server::playing_actions::PlayingActionType;
use server::profiling::start_profiling;
use server::{
    action::{self, Action},
    ai_actions,
    game::GameState,
    game_setup,
    utils::{Rng, Shuffle},
};
use std::env;

mod common;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_random_actions() {
    let iterations = env::var("ITERATIONS")
        .map(|v| v.parse::<usize>().ok())
        .ok()
        .flatten()
        .unwrap_or(100);

    start_profiling();
    let num_cores = num_cpus::get();
    let cache = Cache::new();

    let mut rng = Rng::new();
    let mut iteration = 0;
    loop {
        let mut handles = Vec::new();
        for _ in 0..num_cores {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let cache = cache.clone();
            let handle = task::spawn(async move { random_actions_iteration(thread_rng, cache) });
            handles.push(handle);
        }
        for handle in handles {
            handle.await;
            iteration += 1;
            if iteration % 100 == 0 {
                println!("Iterations: {iteration}");
            }
        }
        if iteration >= iterations {
            break;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn random_actions_iteration(mut rng: Rng, cache: Cache) {
    let seed = rng.range(0, 10_usize.pow(15)).to_string();
    let mut game =
        game_setup::setup_game_with_cache(&GameSetupBuilder::new(2).seed(seed).build(), cache);
    game.context = GameContext::AI;
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

#[cfg(not(target_arch = "wasm32"))]
fn no_action_available(game: &Game) -> Action {
    use server::action::Action;
    use server::game::GameState;
    use server::movement::MovementAction;
    use server::playing_actions::PlayingAction;
    if matches!(game.state, GameState::Movement(_)) {
        return Action::Movement(MovementAction::Stop);
    }
    Action::Playing(PlayingAction::EndTurn)
}

#[cfg(not(target_arch = "wasm32"))]
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
            let destinations = possible_move_routes(p, game, &unit_ids, pos, None);
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
