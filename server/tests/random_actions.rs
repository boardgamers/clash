use server::{
    action::{self, Action},
    ai_actions,
    game::GameState,
    game_setup,
    playing_actions::PlayingAction,
    utils::{Rng, Shuffle},
};

const ITERATIONS: usize = 10;

#[test]
fn test_random_actions() {
    let mut rng = Rng::new();
    for _ in 0..ITERATIONS {
        //todo: multithreading
        let seed = rng.range(0, 10_usize.pow(15)).to_string();
        let mut game = game_setup::setup_game(2, seed, true);
        game.supports_undo = false;
        loop {
            if matches!(game.state, GameState::Finished) {
                break;
            }
            let player_index = game.active_player();
            //todo: include movement action
            let actions = ai_actions::get_available_actions(&game);
            let actions = actions
                .into_iter()
                .flat_map(|(_, actions)| actions)
                .collect::<Vec<Action>>();
            let action = actions
                .take_random_element(&mut rng)
                .unwrap_or(Action::Playing(PlayingAction::EndTurn));
            game = action::execute_action(game, action, player_index);
        }
    }
}
