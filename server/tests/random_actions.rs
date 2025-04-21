use async_std::task;
use itertools::Itertools;
use server::ai_actions::AiActions;
use server::profiling::start_profiling;
use server::{
    action::{self, Action},
    game::GameState,
    game_setup,
    playing_actions::PlayingAction,
    utils::{Rng, Shuffle},
};

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
        actions.extend(get_movement_actions(game));

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
