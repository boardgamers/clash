use async_std::task;
use server::ai_actions::AiActions;
use server::game::Game;
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
        }
        iterations += num_cores;
        if iterations >= ITERATIONS {
            break;
        }
    }
}

fn random_actions_iterations(mut rng: Rng) {
    let seed = rng.range(0, 10_usize.pow(15)).to_string();
    let mut game = game_setup::setup_game(2, seed, true);
    game.supports_undo = false;
    build_adjacent_cities(&mut game);
    let mut ai_actions = AiActions::new();
    loop {
        if matches!(game.state, GameState::Finished) {
            break;
        }
        let player_index = game.active_player();
        //todo: include movement action
        let actions = ai_actions.get_available_actions(&game);
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

fn build_adjacent_cities(_game: &mut Game) {
    // only until move works
    // this way we can test constructing buildings

    // todo makes it too slow
    // for i in game.human_players(0) {
    //     let city = &game.player(i).cities[0];
    //     let pos = city
    //         .position
    //         .neighbors()
    //         .into_iter()
    //         .find(|p| game.map.get(*p) == Some(&Terrain::Barren))
    //         .expect("no barren space");
    //     game.player_mut(i).cities.push(City::new(i, pos));
    // }
}
