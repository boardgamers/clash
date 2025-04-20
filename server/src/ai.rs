extern crate num_cpus;
use std::time::Duration;

use tokio::runtime::Runtime;

use crate::ai_actions::AiActions;
use crate::{
    action::{self, Action, ActionType}
    ,
    game::{Game, GameData, GameState},
    playing_actions::PlayingAction,
    position::Position,
    utils::{self, Rng},
};

const ACTION_SCORE_WEIGHTING: f64 = 1.0;
const ADAPTIVE_DIFFICULTY_SCORE_THRESHOLD: f64 = 10.0;

pub struct AI {
    rng: Rng,
    pub difficulty: f64,
    pub thinking_time: Duration,
    pub adaptive_difficulty: bool,
    active_missions: Vec<Mission>,
    pub ai_actions: AiActions,
}

impl AI {
    ///
    ///
    /// # Panics
    ///
    /// Panics if the difficulty is not between 0 and 1
    ///
    ///
    /// # Panics
    ///
    /// Panics if the difficulty is not between 0 and 1
    #[must_use]
    pub fn new(difficulty: f64, thinking_time: Duration, adaptive_difficulty: bool) -> Self {
        assert!((0.0..=1.0).contains(&difficulty));
        let rng = Rng::new();
        AI {
            rng,
            difficulty,
            thinking_time,
            adaptive_difficulty,
            active_missions: Vec::new(),
            ai_actions: AiActions::new(),
        }
    }

    /// Returns the next action for the AI to take.
    ///
    /// # Panics
    ///
    /// May panic if the game is in an invalid state or if `ai_actions` provides an invalid action.
    /// Returns the next action for the AI to take.
    ///
    /// # Panics
    ///
    /// May panic if the game is in an invalid state or if `ai_actions` provides an invalid action.
    pub fn next_action(&mut self, game: &Game) -> Action {
        //todo: handle movement phase actions
        let actions = self.ai_actions.get_available_actions(game);
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
        if actions.len() == 1 {
            return actions
                .into_iter()
                .next()
                .expect("there should be 1 available action")
                .1;
        }

        let runtime = Runtime::new().expect("failed to create runtime");

        //todo: dynamic thinking time
        let thinking_time_per_action = self.thinking_time / actions.len() as u32;

        //todo: handle going into movement phase

        let difficulty_factor = if self.difficulty >= 1.0 - f64::EPSILON {
            1.0
        } else {
            self.difficulty / (1.0 - self.difficulty)
        };
        let evaluations = actions
            .iter()
            .map(|(action_group, action)| {
                runtime
                    .block_on(evaluate_action(
                        game,
                        action,
                        action_group,
                        &mut self.rng,
                        &thinking_time_per_action,
                    ))
                    .powf(difficulty_factor)
            })
            .collect::<Vec<f64>>();

        let index = if self.difficulty >= 1.0 - f64::EPSILON {
            evaluations
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).expect("floating point error"))
                .expect("there are no possible actions")
                .0
        } else {
            utils::weighted_random_selection(&evaluations, &mut self.rng)
        };

        if self.difficulty > 0.0 + f64::EPSILON {
            let final_evaluation = evaluations
                .into_iter()
                .nth(index)
                .expect("there are no possible actions")
                .powf(1.0 / difficulty_factor);
            println!("average final relative score: {final_evaluation}");
            if self.adaptive_difficulty {
                if final_evaluation > ADAPTIVE_DIFFICULTY_SCORE_THRESHOLD {
                    println!("increasing difficulty");
                    self.increase_difficulty();
                } else if final_evaluation < -ADAPTIVE_DIFFICULTY_SCORE_THRESHOLD {
                    println!("decreasing difficulty");
                    self.decrease_difficulty();
                }
            }
        } else {
            println!("can't get action score or adapt difficulty because the AI's difficulty is 0");
        }

        actions
            .into_iter()
            .nth(index)
            .expect("there are no possible actions")
            .1
    }

    fn increase_difficulty(&mut self) {
        if self.difficulty < 1.0 - f64::EPSILON {
            self.difficulty += 0.25;
            self.difficulty = self.difficulty.max(1.0);
            return;
        }
        self.thinking_time += Duration::from_millis(500);
        self.thinking_time = self.thinking_time.max(Duration::from_millis(5000));
    }

    fn decrease_difficulty(&mut self) {
        if self.thinking_time > Duration::from_millis(250) {
            self.thinking_time -= Duration::from_millis(250);
            self.thinking_time = self.thinking_time.min(Duration::from_millis(250));
            return;
        }
        self.difficulty -= 0.25;
        self.difficulty = self.difficulty.min(0.25);
    }
}

async fn evaluate_action(
    game: &Game,
    action: &Action,
    action_group: &ActionType,
    rng: &mut Rng,
    thinking_time: &Duration,
) -> f64 {
    let action_score = get_action_score(game, action);
    let action_group_score = get_action_group_score(game, action_group);
    let player_index = game.active_player();
    let mut game = game.clone();
    game.supports_undo = false;
    let game = action::execute_action(game, action.clone(), player_index);
    let mut iterations = 0;
    let start_time = std::time::Instant::now();
    let mut score = 0.0;
    loop {
        let mut handles = Vec::new();
        let num_cores = num_cpus::get();
        for _ in 0..num_cores {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let new_game = game.cloned_data();
            let handle =
                tokio::spawn(async move { monte_carlo_score(thread_rng, player_index, new_game) });
            handles.push(handle);
        }
        for handle in handles {
            score += handle.await.expect("multi-threading error");
        }
        iterations += num_cores;
        if start_time.elapsed() >= *thinking_time {
            break;
        }
    }
    println!(
        "Monte Carlo iterations: {iterations}. Score: {}, {action_score}, {action_group_score}",
        score / iterations as f64
    );
    (score / iterations as f64) * action_score * action_group_score
}

fn monte_carlo_score(mut rng: Rng, player_index: usize, game_data: GameData) -> f64 {
    let mut game = Game::from_data(game_data);
    game.supports_undo = false;
    let mut ai = AiActions::new();
    let new_game = monte_carlo_run(&mut ai, game, &mut rng);
    let ai_score = new_game.players[player_index].victory_points(&new_game) as f64;
    let mut max_opponent_score = 0.0;
    for (i, player) in new_game.players.iter().enumerate() {
        if i != player_index {
            let opponent_score = player.victory_points(&new_game) as f64;
            if opponent_score > max_opponent_score {
                max_opponent_score = opponent_score;
            }
        }
    }
    ai_score - max_opponent_score
}

fn monte_carlo_run(ai: &mut AiActions, mut game: Game, rng: &mut Rng) -> Game {
    loop {
        if matches!(game.state, GameState::Finished) {
            return game;
        }
        let current_player = game.active_player();
        let action = choose_monte_carlo_action(ai, &game, rng);
        game = action::execute_action(game, action, current_player);
    }
}

fn choose_monte_carlo_action(ai: &mut AiActions, game: &Game, rng: &mut Rng) -> Action {
    let action_groups = ai.get_available_actions(game);
    if action_groups.is_empty() {
        return Action::Playing(PlayingAction::EndTurn);
    }
    if action_groups.len() == 1 && action_groups[0].1.len() == 1 {
        return action_groups
            .into_iter()
            .next()
            .expect("there are no possible actions")
            .1
            .into_iter()
            .next()
            .expect("there are no possible actions");
    }
    let group_evaluations = action_groups
        .iter()
        .map(|(action_group, _)| get_action_group_score(game, action_group))
        .collect::<Vec<f64>>();
    let group_index = utils::weighted_random_selection(&group_evaluations, rng);
    let action_group = action_groups
        .into_iter()
        .nth(group_index)
        .expect("index out of bounds")
        .1;
    let action_evaluations = action_group
        .iter()
        .map(|action| get_action_score(game, action))
        .collect::<Vec<f64>>();
    let action_index = utils::weighted_random_selection(&action_evaluations, rng);
    action_group
        .into_iter()
        .nth(action_index)
        .expect("index out of bounds")
}

fn get_action_score(game: &Game, action: &Action) -> f64 {
    1_f64.powf(ACTION_SCORE_WEIGHTING)
}

fn get_action_group_score(game: &Game, action_group: &ActionType) -> f64 {
    1_f64.powf(ACTION_SCORE_WEIGHTING)
}

struct Mission {
    units_under_management: Vec<usize>,
    target: Position,
    mission_type: MissionType,
}

impl Mission {
    fn new(
        units_under_management: Vec<usize>,
        target: Position,
        mission_type: MissionType,
    ) -> Self {
        Self {
            units_under_management,
            target,
            mission_type,
        }
    }

    fn priority(&self, game: &Game) -> f64 {
        match self.mission_type {
            MissionType::Explore => 0.5,
            MissionType::DefendCity => 1.0,
            MissionType::FoundCity => 1.5,
            MissionType::CapturePlayerCity => 1.0,
            MissionType::CaptureBarbarianCamp => 1.0,
            MissionType::FightPlayerForces { .. } => 1.0,
            MissionType::FightBarbarians { .. } => 1.0,
            MissionType::FightPirates { .. } => 1.0,
            MissionType::Transport => 1.0,
        }
    }

    fn update(&mut self, game: &Game) {}

    fn is_complete(&self, game: &Game) -> bool {
        false
    }

    fn next_movement(&self, game: &Game) -> Position {
        todo!()
    }
}

enum MissionType {
    Explore,
    DefendCity,
    FoundCity,
    CapturePlayerCity,
    CaptureBarbarianCamp,
    FightPlayerForces {
        player_index: usize,
        units: Vec<usize>,
    },
    FightBarbarians {
        units: Vec<usize>,
    },
    FightPirates {
        units: Vec<usize>,
    },
    Transport,
}

/// Returns the win probability of each player in the game in the order listed in the game.
///
/// # Panics
///
/// Panics if the game is in an invalid state.
#[must_use]
pub fn evaluate_position(ai: &mut AiActions, game: &Game, evaluation_time: Duration) -> Vec<f64> {
    let mut rng = Rng::new();
    let start_time = std::time::Instant::now();
    let mut wins = vec![0; game.players.len()];
    let mut iterations = 0;
    loop {
        let new_game = monte_carlo_run(ai, game.clone(), &mut rng);
        let max_score = new_game
            .players
            .iter()
            .map(|player| player.victory_points(&new_game) as f64)
            .max_by(|a, b| a.partial_cmp(b).expect("floating point error"))
            .expect("there are no players");
        for (i, player) in new_game.players.iter().enumerate() {
            let score = player.victory_points(&new_game) as f64;
            if (score - max_score).abs() < f64::EPSILON {
                wins[i] += 1;
            }
        }
        iterations += 1;
        if start_time.elapsed() >= evaluation_time {
            return wins
                .iter()
                .map(|win| *win as f64 / iterations as f64)
                .collect();
        }
    }
}

/// Returns a score between 0 and 1 for the given action. with 0 being the worst possible action and 1 being the best.
///
/// # Panics
///
/// Panics if the game is in an invalid state.
#[must_use]
pub fn rate_action(ai: &mut AI, game: &Game, action: &Action, evaluation_time: Duration) -> f64 {
    let all_actions = ai.ai_actions.get_available_actions(game);
    let all_actions = all_actions
        .into_iter()
        .flat_map(|(action_group, actions)| {
            actions
                .into_iter()
                .map(|action| (action_group.clone(), action))
                .collect::<Vec<(ActionType, Action)>>()
        })
        .collect::<Vec<(ActionType, Action)>>();
    if all_actions.is_empty() {
        return 1.0;
    }
    let action_index = all_actions
        .iter()
        .position(|(_, a)| a == action)
        .expect("action not found in available actions");
    let mut rng = Rng::new();
    let runtime = Runtime::new().expect("failed to create runtime");
    let thinking_time_per_action = evaluation_time / all_actions.len() as u32;
    let evaluations = all_actions
        .iter()
        .map(|(action_group, action)| {
            runtime.block_on(evaluate_action(
                game,
                action,
                action_group,
                &mut rng,
                &thinking_time_per_action,
            ))
        })
        .collect::<Vec<f64>>();
    let max_evaluation = evaluations
        .iter()
        .max_by(|a, b| a.partial_cmp(b).expect("floating point error"))
        .expect("there are no possible actions");
    let min_evaluation = evaluations
        .iter()
        .min_by(|a, b| a.partial_cmp(b).expect("floating point error"))
        .expect("there are no possible actions");
    evaluations[action_index] - *min_evaluation / (*max_evaluation - *min_evaluation)
}
