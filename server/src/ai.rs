use core::panic;
use std::time::Duration;

use tokio::runtime::Runtime;

use crate::{
    action::{self, Action, ActionType},
    ai_actions,
    game::{Game, GameData, GameState},
    movement::{MoveUnits, MovementAction},
    playing_actions::PlayingAction,
    position::Position,
    resource_pile::ResourcePile,
    unit::Unit,
    utils::{self, Rng},
};

const ACTION_SCORE_WEIGHTING: f64 = 1.0;
const ADAPTIVE_DIFFICULTY_SCORE_THRESHOLD: f64 = 10.0;
const PARALLELIZATION: usize = 24;

pub struct AI {
    rng: Rng,
    pub difficulty: f64,
    pub thinking_time: Duration,
    pub adaptive_difficulty: bool,
    active_missions: Vec<Mission>,
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
                        self.active_missions.clone(),
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
    mut active_missions: Vec<Mission>,
) -> f64 {
    let action_score = get_action_score(game, action, &active_missions);
    let action_group_score = get_action_group_score(game, action_group, &active_missions);
    let player_index = game.active_player();
    let mut game = game.clone();
    game.supports_undo = false;
    let game = action::execute_action(game, action.clone(), player_index);
    for mission in &mut active_missions {
        mission.update(&game);
    }
    let mut iterations = 0;
    let start_time = std::time::Instant::now();
    let mut score = 0.0;
    loop {
        let mut handles = Vec::new();
        for _ in 0..PARALLELIZATION {
            rng.seed = rng.seed.wrapping_add(1);
            rng.next_seed();
            let thread_rng = rng.clone();
            let new_game = game.cloned_data();
            let new_active_missions = active_missions.clone();
            let handle = tokio::spawn(async move {
                monte_carlo_score(thread_rng, player_index, new_game, new_active_missions)
            });
            handles.push(handle);
        }
        for handle in handles {
            score += handle.await.expect("multi-threading error");
        }
        iterations += PARALLELIZATION;
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

fn update_active_missions(active_missions: &mut Vec<Mission>, player_index: usize, game: &Game) {
    let missions = active_missions.clone();
    active_missions.retain(|mission| !mission.is_complete(game, &missions));
    let new_missions = allocate_units(
        &game.players[player_index].units,
        game,
        player_index,
        &*active_missions,
    );
    active_missions.extend(new_missions);
}

fn monte_carlo_score(
    mut rng: Rng,
    player_index: usize,
    game_data: GameData,
    active_missions: Vec<Mission>,
) -> f64 {
    let new_game = monte_carlo_run(Game::from_data(game_data), &mut rng, active_missions);
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

fn monte_carlo_run(mut game: Game, rng: &mut Rng, mut active_missions: Vec<Mission>) -> Game {
    loop {
        if matches!(game.state, GameState::Finished) {
            return game;
        }
        let current_player = game.active_player();
        let action = choose_monte_carlo_action(&game, rng, &active_missions);
        game = action::execute_action(game, action, current_player);
    }
}

fn choose_monte_carlo_action(game: &Game, rng: &mut Rng, active_missions: &[Mission]) -> Action {
    let action_groups = ai_actions::get_available_actions(game);
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

fn get_action_score(game: &Game, action: &Action, active_missions: &[Mission]) -> f64 {
    match action {
        Action::Playing(action) => 1.0,
        Action::Movement(action) => {
            let mission = active_missions
                .iter()
                .find(|mission| mission.action == *action)
                .expect("movement action is not part of any active mission");
            mission.priority(game, active_missions)
        }
        Action::Response(action) => 1.0,
        _ => panic!("invalid ai action"),
    }
    .powf(ACTION_SCORE_WEIGHTING)
}

fn get_action_group_score(
    game: &Game,
    action_group: &ActionType,
    active_missions: &[Mission],
) -> f64 {
    match action_group {
        ActionType::Playing(action) => 1.0,
        ActionType::Movement => active_missions.iter().fold(0.0, |acc, mission| {
            acc + mission.priority(game, active_missions)
        }),
        _ => 1.0,
    }
    .powf(ACTION_SCORE_WEIGHTING)
}

#[derive(Clone)]
struct Mission {
    units_under_management: Vec<u32>,
    player_index: usize,
    mission_type: MissionType,
    target: Position,
    current_location: Position,
    action: MovementAction,
    id: u32,
}

impl Mission {
    fn new(
        units_under_management: Vec<u32>,
        target: Position,
        mission_type: MissionType,
        game: &Game,
        player_index: usize,
        id: u32,
    ) -> Self {
        let current_location = game.players[player_index]
            .get_unit(units_under_management[0])
            .position;
        let mut mission = Self {
            units_under_management,
            player_index,
            target,
            mission_type,
            current_location,
            action: MovementAction::Stop,
            id,
        };
        mission.action = mission.next_movement(game);
        mission
    }

    fn priority(&self, game: &Game, missions: &[Mission]) -> f64 {
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
            MissionType::JoinMission(id) => {
                let mission = missions
                    .iter()
                    .find(|mission| mission.id == id)
                    .expect("mission not found");
                let base_priority = mission.priority(game, missions);
                let distance = self.current_location.distance(mission.current_location);
                (base_priority - distance as f64 * 0.1).min(0.1)
            }
        }
    }

    fn update(&mut self, game: &Game) {}

    fn is_complete(&self, game: &Game, missions: &[Mission]) -> bool {
        self.current_location == self.target
            || self.units_under_management.is_empty()
            || match self.mission_type {
                MissionType::Explore => todo!(),
                MissionType::DefendCity => game.players[self.player_index]
                    .try_get_city(self.target)
                    .is_none(),
                MissionType::FoundCity => game.players[self.player_index]
                    .try_get_city(self.target)
                    .is_some(),
                MissionType::CapturePlayerCity => todo!(),
                MissionType::CaptureBarbarianCamp => todo!(),
                MissionType::FightPlayerForces { .. } => false,
                MissionType::FightBarbarians { .. } => false,
                MissionType::FightPirates { .. } => false,
                MissionType::Transport => false,
                MissionType::JoinMission(id) => {
                    let mission = missions
                        .iter()
                        .find(|mission| mission.id == id)
                        .expect("mission not found");
                    mission.is_complete(game, missions)
                }
            }
    }

    fn next_movement(&self, game: &Game) -> Vec<MovementAction> {
        let next_position = self
            .current_location
            .next_position_in_path(&self.target)
            .expect("missions is at it's target location");
        let carrier = game.players[self.player_index]
            .get_unit(self.units_under_management[0])
            .carrier_id;
        let payment = self.movement_payment(game);
        MovementAction::Move(MoveUnits::new(
            self.units_under_management.clone(),
            next_position,
            carrier,
            payment,
        ))
    }

    fn movement_payment(&self, game: &Game) -> ResourcePile {
        todo!()
    }
}

#[derive(Clone)]
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
    JoinMission(u32),
}

fn allocate_units(
    units: &[Unit],
    game: &Game,
    player_index: usize,
    active_missions: &[Mission],
) -> Vec<Mission> {
    todo!()
}

/// Returns the win probability of each player in the game in the order listed in the game.
///
/// # Panics
///
/// Panics if the game is in an invalid state.
#[must_use]
pub fn evaluate_position(game: &Game, evaluation_time: Duration) -> Vec<f64> {
    let mut rng = Rng::new();
    let start_time = std::time::Instant::now();
    let mut wins = vec![0; game.players.len()];
    let mut iterations = 0;
    loop {
        let new_game = monte_carlo_run(game.clone(), &mut rng, Vec::new());
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
pub fn rate_action(game: &Game, action: &Action, evaluation_time: Duration) -> f64 {
    let all_actions = ai_actions::get_available_actions(game);
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
                Vec::new(),
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
