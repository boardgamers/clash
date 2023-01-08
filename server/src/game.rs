use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    content::civilizations,
    player::{Player, PlayerData},
    playing_actions::PlayingAction::*,
    status_phase_actions::StatusPhaseAction,
};

use GameState::*;
use StatusPhaseState::*;

const DICE_ROLL_BUFFER: u32 = 200;
const AGES: u32 = 6;

pub struct Game {
    pub state: GameState,
    pub players: Vec<Player>,
    pub starting_player: usize,
    pub current_player: usize,
    pub log: Vec<LogItem>,
    pub played_limited_actions: Vec<String>,
    pub actions_left: u32,
    pub round: u32,
    pub age: u32,
    pub messages: Vec<String>,
    pub dice_roll_outcomes: Vec<u8>,
    dropped_players: Vec<usize>,
}

impl Game {
    pub fn new(player_amount: usize, seed: String) -> Self {
        let seed_length = seed.len();
        let seed = if seed_length < 32 {
            seed + &" ".repeat(32 - seed_length)
        } else {
            String::from(&seed[..32])
        };
        let seed = seed
            .as_bytes()
            .try_into()
            .expect("seed should be of length 32");
        let mut random_number_generator = StdRng::from_seed(seed);

        let mut players = Vec::new();
        let mut civilizations = civilizations::get_civilizations();
        for _ in 0..player_amount {
            let civilization = random_number_generator.gen_range(0..civilizations.len());
            players.push(Player::new(civilizations.remove(civilization)));
        }

        let starting_player = random_number_generator.gen_range(0..players.len());
        let mut dice_roll_outcomes = Vec::new();
        for _ in 0..DICE_ROLL_BUFFER {
            dice_roll_outcomes.push(random_number_generator.gen_range(1..=12));
        }

        Self {
            state: Playing,
            players,
            starting_player,
            current_player: starting_player,
            log: Vec::new(),
            played_limited_actions: Vec::new(),
            actions_left: 3,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            dice_roll_outcomes,
            dropped_players: Vec::new(),
        }
    }

    pub fn from_json(json: &str) -> Self {
        Self::from_data(
            serde_json::from_str(json).expect("API call should receive valid game data json"),
        )
    }

    pub fn json(self) -> String {
        serde_json::to_string(&self.data()).expect("game data should be valid json")
    }

    fn from_data(data: GameData) -> Self {
        Self {
            state: data.state,
            players: data.players.into_iter().map(Player::from_data).collect(),
            starting_player: data.current_player,
            current_player: data.current_player,
            actions_left: data.actions_left,
            log: data.log,
            played_limited_actions: data.played_limited_actions,
            round: data.round,
            age: data.age,
            messages: data.messages,
            dice_roll_outcomes: data.dice_roll_outcomes,
            dropped_players: data.dropped_players,
        }
    }

    fn data(self) -> GameData {
        GameData {
            state: self.state,
            players: self
                .players
                .into_iter()
                .map(|player| player.data())
                .collect(),
            starting_player: self.starting_player,
            current_player: self.current_player,
            log: self.log,
            played_limited_actions: self.played_limited_actions,
            actions_left: self.actions_left,
            round: self.round,
            age: self.age,
            messages: self.messages,
            dice_roll_outcomes: self.dice_roll_outcomes,
            dropped_players: self.dropped_players,
        }
    }

    pub fn get_next_dice_roll(&mut self) -> u8 {
        if self.dice_roll_outcomes.is_empty() {
            println!("ran out of predetermined dice roll outcomes, unseeded rng is no being used");
            return rand::thread_rng().gen_range(1..=12);
        }
        self.dice_roll_outcomes
            .pop()
            .expect("dice roll outcomes should not be empty")
    }

    pub fn execute_action(&mut self, action: String, player_index: usize) {
        if let StatusPhase(phase) = self.state.clone() {
            self.execute_status_phase_action(action, phase, player_index);
            return;
        }
        self.execute_playing_action(action, player_index);
    }

    fn execute_playing_action(&mut self, action: String, player_index: usize) {
        let playing_action =
            serde_json::from_str(&action).expect("action should be valid playing action json");
        self.log.push(LogItem::PlayingAction(action));
        let action = playing_action;
        if matches!(action, EndTurn) {
            self.next_turn();
            return;
        }
        let free_action = action.action_type().free;
        if !free_action && self.actions_left == 0 {
            panic!("Illegal action");
        }
        if let Custom { name, .. } = &action {
            if self.played_limited_actions.contains(name) {
                panic!("Illegal action");
            }
            let action_type = action.action_type();
            if action_type.once_per_turn {
                self.played_limited_actions.push(name.clone());
            }
        }
        let mut player = self.players.remove(player_index);
        action.execute(&mut player, self);
        self.players.insert(player_index, player);
        if !free_action {
            self.actions_left -= 1;
        }
    }

    fn execute_status_phase_action(
        &mut self,
        action: String,
        phase: StatusPhaseState,
        player_index: usize,
    ) {
        let action = StatusPhaseAction::new(action, phase.clone());
        self.log.push(LogItem::StatusPhaseAction(
            serde_json::to_string(&action).expect("status phase action should be serializable"),
        ));
        let mut player = self.players.remove(player_index);
        action.execute(&mut player, self);
        self.players.insert(player_index, player);
        if matches!(phase, DetermineFirstPlayer) {
            self.next_age();
            return;
        }
        self.current_player += 1;
        self.current_player %= self.players.len();
        if self.current_player == self.starting_player {
            match phase {
                CompleteObjectives => self.state = StatusPhase(FreeAdvance),
                FreeAdvance => {
                    self.state = StatusPhase(RaseSize1City);
                    //todo! draw cards
                }
                RaseSize1City => self.state = StatusPhase(ChangeGovernmentType),
                ChangeGovernmentType => {
                    self.state = StatusPhase(DetermineFirstPlayer);
                    let mut potential_deciding_players = Vec::new();
                    let mut best_total = 0;
                    for (i, player) in self.players.iter().enumerate() {
                        let total =
                            player.resources().mood_tokens + player.resources().culture_tokens;
                        if total > best_total {
                            best_total = total;
                        }
                        if total == best_total {
                            potential_deciding_players.push(i);
                        }
                    }
                    self.current_player = potential_deciding_players.into_iter().min_by_key(|&index| (index as isize - self.starting_player as isize) % self.players.len() as isize
                ).expect("there should at least be one player with the most mood and culture tokens");
                }
                DetermineFirstPlayer => {
                    unreachable!("function should return early with this action")
                }
            }
        }
        if self.dropped_players.contains(&self.current_player) {
            self.current_player += 1;
            self.current_player %= self.players.len()
        }
    }

    fn next_turn(&mut self) {
        self.actions_left = 3;
        self.played_limited_actions = Vec::new();
        self.current_player += 1;
        self.current_player %= self.players.len();
        if self.current_player == self.starting_player {
            self.next_round();
        }
        if self.dropped_players.contains(&self.current_player) {
            self.current_player += 1;
            self.current_player %= self.players.len()
        }
    }

    fn next_round(&mut self) {
        self.round += 1;
        if self.round > 3 {
            self.round = 1;
            self.enter_status_phase();
        }
    }

    fn enter_status_phase(&mut self) {
        if self.players.iter().any(|player| player.cities.is_empty()) {
            self.end_game();
        }
        self.state = StatusPhase(ChangeGovernmentType);
    }

    fn next_age(&mut self) {
        self.state = Playing;
        self.age += 1;
        if self.age > AGES {
            self.end_game();
            return;
        }
        self.messages.push(format!("Age {} has started", self.age));
    }

    fn end_game(&mut self) {
        self.state = Finished;
        self.add_message("Game has ended");
    }

    fn add_message(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }

    pub fn drop_player(&mut self, player_index: usize) {
        self.dropped_players.push(player_index);
        if self.current_player == player_index {
            self.current_player += 1;
            self.current_player %= self.players.len();
        }
    }

    pub fn get_player(&mut self, name: &str) -> Option<&mut Player> {
        let position = self
            .players
            .iter()
            .position(|player| player.name() == name)?;
        Some(&mut self.players[position])
    }
}

#[derive(Serialize, Deserialize)]
struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    starting_player: usize,
    current_player: usize,
    log: Vec<LogItem>,
    played_limited_actions: Vec<String>,
    actions_left: u32,
    round: u32,
    age: u32,
    messages: Vec<String>,
    dice_roll_outcomes: Vec<u8>,
    dropped_players: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum GameState {
    Playing,
    StatusPhase(StatusPhaseState),
    Finished,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    RaseSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum LogItem {
    PlayingAction(String),
    StatusPhaseAction(String),
}
