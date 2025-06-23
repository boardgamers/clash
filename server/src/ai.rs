use core::panic;
use std::time::Duration;
use std::vec;

use itertools::Itertools;
use num_cpus;
use tokio::runtime::Runtime;

use crate::ai_actions::AiActions;
use crate::cache::Cache;
use crate::game::GameContext;
use crate::game_data::GameData;
use crate::movement::MovementAction;
use crate::{
    action::{self, Action, ActionType},
    ai_missions::ActiveMissions,
    game::{Game, GameState},
    playing_actions::{PlayingAction, PlayingActionType},
    utils::{self, Rng},
};

pub const ACTION_SCORE_WEIGHTING: f64 = 0.0;
const ADAPTIVE_DIFFICULTY_SCORE_THRESHOLD: f64 = 10.0;
const ALLOCATE_UNITS_EVALUATION_TIME: f64 = 0.1;
const PRUNING_ITERATIONS: usize = 3;

pub struct AI {
    rng: Rng,
    pub difficulty: f64,
    pub thinking_time: Duration,
    pub adaptive_difficulty: bool,
    active_missions: ActiveMissions,
    ai_actions: AiActions,
}

impl AI {
    /// Returns an instance of an AI which takes control of one of the players in the game.
    ///
    /// # Panics
    ///
    /// Panics if the difficulty is not between 0 and 1
    #[must_use]
    pub fn new(
        difficulty: f64,
        thinking_time: Duration,
        adaptive_difficulty: bool,
        starting_game: &Game,
        player_index: usize,
    ) -> Self {
        assert!((0.0..=1.0).contains(&difficulty));
        let rng = Rng::new();
        let mut missions_rng = rng.clone();
        missions_rng.seed = missions_rng.seed.wrapping_add(1);
        missions_rng.next_seed();
        let starting_units = starting_game.players[player_index].units.len();

        AI {
            rng,
            difficulty,
            thinking_time,
            adaptive_difficulty,
            active_missions: ActiveMissions::new(
                starting_game,
                player_index,
                &mut missions_rng,
                Some((
                    thinking_time.mul_f64(ALLOCATE_UNITS_EVALUATION_TIME) * starting_units as u32,
                    difficulty,
                )),
            ),
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
    /// Panics if it's not the AI's turn
    pub fn next_action(&mut self, game: &Game) -> Action {
        assert_eq!(game.active_player(), self.active_missions.player_index);
        let start_time = std::time::Instant::now();

        if can_move(game, self.active_missions.player_index) {
            let idle_units = self.active_missions.idle_units.len();
            self.active_missions.update(
                game,
                &mut self.rng,
                Some((
                    self.thinking_time.mul_f64(ALLOCATE_UNITS_EVALUATION_TIME) * idle_units as u32,
                    self.difficulty,
                )),
            );
        }

        let mut actions = get_actions(&mut self.ai_actions, game, &self.active_missions);
        if actions.is_empty() {
            return forced_action(game);
        }
        if actions.len() == 1 {
            return actions
                .into_iter()
                .next()
                .expect("there should be 1 available action")
                .1;
        }

        let runtime = Runtime::new().expect("failed to create runtime");

        let players_active_missions = self
            .active_missions
            .get_players_active_missions(game, &mut self.rng);
        let difficulty_factor = difficulty_factor(self.difficulty);

        let mut evaluations = vec![1.0; actions.len()];
        for i in 0..PRUNING_ITERATIONS {
            if start_time.elapsed() >= self.thinking_time {
                break;
            }
            let time_remaining = self.thinking_time - start_time.elapsed();
            let thinking_time_per_action =
                time_remaining / actions.len() as u32 / (PRUNING_ITERATIONS - i) as u32;
            for (j, new_evaluation) in actions
                .iter()
                .map(|(action_group, action)| {
                    runtime
                        .block_on(evaluate_action(
                            game,
                            action,
                            action_group,
                            &mut self.rng,
                            thinking_time_per_action,
                            &players_active_missions,
                        ))
                        .powf(difficulty_factor)
                })
                .enumerate()
            {
                evaluations[j] = utils::new_average(evaluations[j], new_evaluation, i);
            }
            let median = utils::median(&evaluations);
            let (new_actions, new_evaluations) = actions
                .into_iter()
                .zip(evaluations.into_iter())
                .filter(|&(_, evaluation)| evaluation >= median)
                .unzip::<(ActionType, Action), f64, Vec<(ActionType, Action)>, Vec<f64>>();
            actions = new_actions;
            evaluations = new_evaluations;
        }

        let chosen_action = if self.difficulty >= 1.0 - f64::EPSILON {
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
                .nth(chosen_action)
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
            .nth(chosen_action)
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
    thinking_time: Duration,
    players_active_missions: &[ActiveMissions],
) -> f64 {
    let player_index = game.active_player();
    let action_score = get_action_score(game, action, &players_active_missions[player_index]);
    let action_group_score =
        get_action_group_score(game, action_group, &players_active_missions[player_index]);
    let game = action::execute_action(game.clone(), action.clone(), player_index);
    let score = get_average_score(
        game,
        player_index,
        rng,
        thinking_time,
        players_active_missions,
    )
    .await;
    println!(
        " -> Monte Carlo score: {score}, action score: {action_score}, action group score: {action_group_score}"
    );
    score * action_score * action_group_score
}

/// Simulates the current game multiple times to the end and returns the average score for the given player relative the best opponent.
///
/// # Panics
///
/// Panics if the game is in an invalid state or if the player index is out of bounds.
pub async fn get_average_score(
    game: Game,
    player_index: usize,
    rng: &mut Rng,
    evaluation_time: Duration,
    players_active_missions: &[ActiveMissions],
) -> f64 {
    let mut iterations = 0;
    let start_time = std::time::Instant::now();
    let mut score = 0.0;
    let num_cores = num_cpus::get();
    loop {
        let mut handles = Vec::new();
        for _ in 0..num_cores {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let new_game = game.cloned_data();
            let new_active_missions = players_active_missions.to_vec();
            let cache = game.cache.clone();
            let handle = tokio::spawn(async move {
                monte_carlo_score(
                    thread_rng,
                    player_index,
                    new_game,
                    new_active_missions,
                    cache,
                )
            });
            handles.push(handle);
        }
        for handle in handles {
            score += handle.await.expect("multi-threading error");
        }
        iterations += num_cores;
        if start_time.elapsed() >= evaluation_time {
            break;
        }
    }
    println!("Monte Carlo iterations: {iterations}");
    score / iterations as f64
}

fn monte_carlo_score(
    mut rng: Rng,
    player_index: usize,
    game_data: GameData,
    players_active_missions: Vec<ActiveMissions>,
    cache: Cache,
) -> f64 {
    let mut ai = AiActions::new();
    let new_game = monte_carlo_run(
        &mut ai,
        game_from_data(game_data, cache),
        &mut rng,
        players_active_missions,
    );
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

fn game_from_data(game_data: GameData, cache: Cache) -> Game {
    Game::from_data(game_data, cache, GameContext::AI)
}

fn monte_carlo_run(
    ai: &mut AiActions,
    mut game: Game,
    rng: &mut Rng,
    mut players_active_missions: Vec<ActiveMissions>,
) -> Game {
    loop {
        if matches!(game.state, GameState::Finished) {
            return game;
        }
        let current_player = game.active_player();
        if can_move(&game, current_player) {
            players_active_missions[current_player].update(&game, rng, None);
        }
        let action =
            choose_monte_carlo_action(ai, &game, rng, &players_active_missions[current_player]);
        game = action::execute_action(game, action, current_player);
    }
}

fn choose_monte_carlo_action(
    ai_actions: &mut AiActions,
    game: &Game,
    rng: &mut Rng,
    active_missions: &ActiveMissions,
) -> Action {
    let action_groups = get_action_groups(ai_actions, game, active_missions);
    if action_groups.is_empty() {
        return forced_action(game);
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
        .map(|(action_group, _)| get_action_group_score(game, action_group, active_missions))
        .collect::<Vec<f64>>();
    let group_index = utils::weighted_random_selection(&group_evaluations, rng);
    let action_group = action_groups
        .into_iter()
        .nth(group_index)
        .expect("index out of bounds")
        .1;
    let action_evaluations = action_group
        .iter()
        .map(|action| get_action_score(game, action, active_missions))
        .collect::<Vec<f64>>();
    let action_index = utils::weighted_random_selection(&action_evaluations, rng);
    action_group
        .into_iter()
        .nth(action_index)
        .expect("index out of bounds")
}

fn forced_action(game: &Game) -> Action {
    if matches!(game.state, GameState::Movement(_)) {
        Action::Movement(MovementAction::Stop)
    } else {
        Action::Playing(PlayingAction::EndTurn)
    }
}

fn get_actions(
    ai_actions: &mut AiActions,
    game: &Game,
    active_missions: &ActiveMissions,
) -> Vec<(ActionType, Action)> {
    let action_groups = get_action_groups(ai_actions, game, active_missions);

    action_groups
        .into_iter()
        .flat_map(|(action_group, actions)| {
            actions
                .into_iter()
                .map(|action| (action_group.clone(), action))
                .collect::<Vec<(ActionType, Action)>>()
        })
        .collect::<Vec<(ActionType, Action)>>()
}

fn get_action_groups(
    ai_actions: &mut AiActions,
    game: &Game,
    active_missions: &ActiveMissions,
) -> Vec<(ActionType, Vec<Action>)> {
    let mut actions = ai_actions.get_available_actions(game);
    if can_move(game, active_missions.player_index) {
        let mission_actions = active_missions
            .missions
            .iter()
            .filter_map(|mission| mission.next_action.as_ref())
            .cloned()
            .map(Action::Movement)
            .collect::<Vec<Action>>();
        if !mission_actions.is_empty() {
            actions.push((ActionType::Movement, mission_actions));
        }
    }
    actions
}

fn can_move(game: &Game, player_index: usize) -> bool {
    PlayingActionType::MoveUnits
        .is_available(game, player_index)
        .is_ok()
}

#[must_use]
pub fn difficulty_factor(difficulty: f64) -> f64 {
    if difficulty >= 1.0 - f64::EPSILON {
        1.0
    } else {
        difficulty / (1.0 - difficulty)
    }
}

fn get_action_score(game: &Game, action: &Action, active_missions: &ActiveMissions) -> f64 {
    match action {
        Action::Playing(_action) => 1.0, //todo
        Action::Movement(action) => {
            let mission = active_missions
                .missions
                .iter()
                .find(|mission| {
                    mission
                        .next_action
                        .as_ref()
                        .is_some_and(|mission_action| mission_action == action)
                })
                .expect("movement action is not part of any active mission");
            mission.priority(game, active_missions)
        }
        Action::Response(_action) => 1.0,
        _ => panic!("invalid ai action"),
    }
    .powf(ACTION_SCORE_WEIGHTING)
}

fn get_action_group_score(
    game: &Game,
    action_group: &ActionType,
    active_missions: &ActiveMissions,
) -> f64 {
    match action_group {
        ActionType::Playing(_action) => 1.0, //todo
        ActionType::Movement => {
            let skip = if active_missions.missions.len() <= 3 {
                0
            } else {
                active_missions.missions.len() - 3
            };
            active_missions
                .missions
                .iter()
                .sorted_by(|a, b| {
                    a.priority(game, active_missions)
                        .total_cmp(&b.priority(game, active_missions))
                })
                .rev()
                .skip(skip)
                .fold(0.0, |acc, mission| {
                    acc + mission.priority(game, active_missions)
                })
        }
        _ => 1.0,
    }
    .powf(ACTION_SCORE_WEIGHTING)
}

/// Returns the win probability of each player in the game in the order listed in the game.
///
/// # Panics
///
/// Panics if the game is in an invalid state.
#[must_use]
pub async fn evaluate_position(game: &Game, evaluation_time: Duration) -> Vec<f64> {
    let mut rng = Rng::new();
    let start_time = std::time::Instant::now();
    let players_active_missions = game
        .players
        .iter()
        .map(|player| ActiveMissions::new(game, player.index, &mut rng, None))
        .collect::<Vec<ActiveMissions>>();
    let mut wins = vec![0; game.players.len()];
    let num_cores = num_cpus::get();
    let mut iterations = 0;
    loop {
        let mut handles = Vec::new();
        for _ in 0..num_cores {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let new_game = game.cloned_data();
            let new_active_missions = players_active_missions.clone();
            let cache = game.cache.clone();
            let handle = tokio::spawn(async move {
                simulate_game(new_game, thread_rng, new_active_missions, cache)
            });
            handles.push(handle);
        }
        for handle in handles {
            let winner = handle.await.expect("multi-threading error");
            wins[winner] += 1;
        }
        iterations += num_cores;
        if start_time.elapsed() >= evaluation_time {
            return wins
                .iter()
                .map(|win| *win as f64 / iterations as f64)
                .collect();
        }
    }
}

fn simulate_game(
    game: GameData,
    mut rng: Rng,
    players_active_missions: Vec<ActiveMissions>,
    cache: Cache,
) -> usize {
    let new_game = monte_carlo_run(
        &mut AiActions::new(),
        game_from_data(game, cache),
        &mut rng,
        players_active_missions,
    );
    let max_score = new_game
        .players
        .iter()
        .map(|player| player.victory_points(&new_game) as f64)
        .max_by(|a, b| a.partial_cmp(b).expect("floating point error"))
        .expect("there are no players");
    for (i, player) in new_game.players.iter().enumerate() {
        let score = player.victory_points(&new_game) as f64;
        if (score - max_score).abs() < f64::EPSILON {
            return i;
        }
    }
    unreachable!("there should be a winner");
}

/// Returns a score between 0 and 1 for the given action. with 0 being the worst possible action and 1 being the best.
///
/// # Panics
///
/// Panics if the game is in an invalid state.
#[must_use]
pub fn rate_action(
    game: &Game,
    player_index: usize,
    action: &Action,
    evaluation_time: Duration,
) -> f64 {
    let mut rng = Rng::new();
    let players_active_missions = game
        .players
        .iter()
        .map(|player| ActiveMissions::new(game, player.index, &mut rng, None))
        .collect::<Vec<ActiveMissions>>();
    let runtime = Runtime::new().expect("failed to create runtime");
    let initial_score = runtime.block_on(get_average_score(
        game.clone(),
        player_index,
        &mut rng,
        evaluation_time / 2,
        &players_active_missions,
    ));
    let new_game = action::execute_action(game.clone(), action.clone(), player_index);
    let new_score = runtime.block_on(get_average_score(
        new_game,
        player_index,
        &mut rng,
        evaluation_time / 2,
        &players_active_missions,
    ));
    new_score - initial_score
}
