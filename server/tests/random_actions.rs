use async_std::task;
use itertools::Itertools;
use server::ai_actions::{get_movement_actions, AiActions};
use server::profiling::start_profiling;
use server::{action::{self, Action}, ai_actions, game::GameState, game_setup, playing_actions::PlayingAction, utils::{Rng, Shuffle}};
use server::action::ActionType;
use server::game::Game;
use server::movement::{move_units_destinations, MoveUnits, MovementAction};
use server::playing_actions::PlayingActionType;

mod common;

const ITERATIONS: usize = 100;

#[tokio::test]
async fn test_random_actions() {
    start_profiling();

    let num_cores = num_cpus::get();
    let mut rng = Rng::new();
    let mut iterations = 0;
    loop {
        let mut handles = Vec::new();
        for _ in 0..num_cores {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let handle = task::spawn(async move { random_actions_iterations(thread_rng) });
            handles.push(handle);
        }
        for handle in handles {
            handle.await;
            iterations += 1;

            if iterations % 100 == 0 {
                println!("Iterations: {}", iterations);
            }
        }
        if iterations >= ITERATIONS {
            break;
        }
    }
}

fn random_actions_iterations(mut rng: Rng) {
    let seed = rng.range(0, 10_usize.pow(15)).to_string();
    let mut game = game_setup::setup_game(2, seed, true);
    game.supports_undo = false;
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
            .unwrap_or(Action::Playing(PlayingAction::EndTurn));

        game = action::execute_action(game.clone(), action.clone(), player_index)
    }
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
                            ai_actions::try_payment(ai_actions, &route.cost, p).map(|pay| {
                                Action::Movement(MovementAction::Move(MoveUnits::new(
                                    unit_ids,
                                    route.destination,
                                    None,
                                    pay,
                                )))
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
