use crate::{
    action::{self, Action},
    game::{Game, GameState},
    utils::{self, Rng},
};

const MONTE_CARLO_ITERATIONS: usize = 100_000;
const ACTION_SCORE_WEIGHTING: f32 = 1.0;

/// 0 < difficulty < 1
pub fn next_action(game: &Game, difficulty: f32, rng: &mut Rng) -> Action {
    let actions = get_all_actions(game);
    let evaluations = actions
        .iter()
        .map(|action| evaluate_action(game, action, rng).powf(difficulty / (1.0 - difficulty)))
        .collect::<Vec<f32>>();

    let index = if difficulty >= 1.0 - f32::EPSILON {
        evaluations
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).expect("floating point error"))
            .expect("there are no possible actions")
            .0
    } else {
        utils::weighted_random_selection(&evaluations, rng)
    };

    actions
        .into_iter()
        .nth(index)
        .expect("there are no possible actions")
}

fn get_all_actions(game: &Game) -> Vec<Action> {
    todo!()
}

fn get_grouped_actions(game: &Game) -> Vec<Vec<Action>> {
    todo!()
}

fn evaluate_action(game: &Game, action: &Action, rng: &mut Rng) -> f32 {
    let action_score = get_action_score(game, action);
    let player_index = game.current_player_index;
    let game = action::execute_action(game.clone(), action.clone(), player_index);
    let mut monte_carlo_score = 0.0;
    for _ in 0..MONTE_CARLO_ITERATIONS {
        let new_game = monte_carlo_run(game.clone(), rng);
        let ai_score = new_game.players[player_index].victory_points(&new_game);
        let mut max_opponent_score = 0.0;
        for (i, player) in new_game.players.iter().enumerate() {
            if i != player_index {
                let opponent_score = player.victory_points(&new_game);
                if opponent_score > max_opponent_score {
                    max_opponent_score = opponent_score;
                }
            }
        }
        monte_carlo_score += (ai_score - max_opponent_score) / MONTE_CARLO_ITERATIONS as f32;
    }
    monte_carlo_score * action_score
}

fn monte_carlo_run(mut game: Game, rng: &mut Rng) -> Game {
    loop {
        if matches!(game.state, GameState::Finished) {
            return game;
        }
        let current_player = game.current_player_index;
        let action = choose_monte_carlo_action(&game, rng);
        game = action::execute_action(game, action, current_player);
    }
}

fn choose_monte_carlo_action(game: &Game, rng: &mut Rng) -> Action {
    let action_groups = get_grouped_actions(game);
    let group_evaluations = action_groups
        .iter()
        .map(|action_group| get_action_group_score(game, action_group))
        .collect::<Vec<f32>>();
    let group_index = utils::weighted_random_selection(&group_evaluations, rng);
    let action_group = action_groups
        .into_iter()
        .nth(group_index)
        .expect("index out of bounds");
    let action_evaluations = action_group
        .iter()
        .map(|action| get_action_score(game, action))
        .collect::<Vec<f32>>();
    let action_index = utils::weighted_random_selection(&action_evaluations, rng);
    action_group
        .into_iter()
        .nth(action_index)
        .expect("index out of bounds")
}

fn get_action_score(game: &Game, action: &Action) -> f32 {
    1_f32.powf(ACTION_SCORE_WEIGHTING)
}

fn get_action_group_score(game: &Game, action_group: &[Action]) -> f32 {
    1_f32.powf(ACTION_SCORE_WEIGHTING)
}
