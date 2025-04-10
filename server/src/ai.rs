use std::time::Duration;

use crate::{
    action::{self, Action, ActionType},
    ai_actions,
    game::{Game, GameState},
    playing_actions::PlayingAction,
    utils::{self, Rng},
};

const ACTION_SCORE_WEIGHTING: f32 = 1.0;

pub struct AI {
    rng: Rng,
    difficulty: f32,
    thinking_time: Duration,
}

impl AI {
    ///
    ///
    /// # Panics
    ///
    /// Panics if the difficulty is not between 0 and 1
    #[must_use]
    pub fn new(difficulty: f32, thinking_time: Duration) -> Self {
        assert!((0.0..=1.0).contains(&difficulty));
        let rng = Rng::new();
        AI {
            rng,
            difficulty,
            thinking_time,
        }
    }

    /// Returns the next action for the AI to take.
    ///
    /// # Panics
    ///
    /// May panic if the game is in an invalid state or if `ai_actions` provides an invalid action.
    pub fn next_action(&mut self, game: &Game) -> Action {
        //todo: handle movement phase actions
        let actions = ai_actions::get_available_actions(game);
        let actions = actions
            .into_iter()
            .flat_map(|(action_group, actions)| {
                actions
                    .into_iter()
                    .map(|action| (action_group.clone(), action))
                    .collect::<Vec<(ActionType, Action)>>()
            })
            .collect::<Vec<(ActionType, Action)>>();
        if actions.is_empty() {
            return Action::Playing(PlayingAction::EndTurn);
        }
        if actions.len() == 1 {
            return actions
                .into_iter()
                .next()
                .expect("there should be 1 available action")
                .1;
        }
        //todo: dynamic thinking time
        let thinking_time_per_action = self.thinking_time / actions.len() as u32;

        //todo: handle going into movement phase

        let evaluations = actions
            .iter()
            .map(|(action_group, action)| {
                evaluate_action(
                    game,
                    action,
                    action_group,
                    &mut self.rng,
                    &thinking_time_per_action,
                )
                .powf(self.difficulty / (1.0 - self.difficulty))
            })
            .collect::<Vec<f32>>();

        let index = if self.difficulty >= 1.0 - f32::EPSILON {
            evaluations
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).expect("floating point error"))
                .expect("there are no possible actions")
                .0
        } else {
            utils::weighted_random_selection(&evaluations, &mut self.rng)
        };

        actions
            .into_iter()
            .nth(index)
            .expect("there are no possible actions")
            .1
    }
}

fn evaluate_action(
    game: &Game,
    action: &Action,
    action_group: &ActionType,
    rng: &mut Rng,
    thinking_time: &Duration,
) -> f32 {
    let action_score = get_action_score(game, action);
    let action_group_score = get_action_group_score(game, action_group);
    let player_index = game.current_player_index;
    let game = action::execute_action(game.clone(), action.clone(), player_index);
    let mut monte_carlo_score = 0.0;
    let mut iterations = 0;
    let start_time = std::time::Instant::now();
    loop {
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
        monte_carlo_score += ai_score - max_opponent_score;
        iterations += 1;
        if start_time.elapsed() >= *thinking_time {
            break;
        }
    }
    monte_carlo_score / iterations as f32 * action_score * action_group_score
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
    let action_groups = ai_actions::get_available_actions(game);
    let group_evaluations = action_groups
        .iter()
        .map(|(action_group, _)| get_action_group_score(game, action_group))
        .collect::<Vec<f32>>();
    let group_index = utils::weighted_random_selection(&group_evaluations, rng);
    let action_group = action_groups
        .into_iter()
        .nth(group_index)
        .expect("index out of bounds")
        .1;
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

fn get_action_group_score(game: &Game, action_group: &ActionType) -> f32 {
    1_f32.powf(ACTION_SCORE_WEIGHTING)
}
