use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    action::PlayingAction::*,
    content::civilizations,
    game_api::UserAction,
    player::{Player, PlayerData},
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
    pub log: Vec<String>,
    pub played_limited_actions: Vec<String>,
    pub actions_left: u32,
    pub round: u32,
    pub age: u32,
    pub messages: Vec<String>,
    pub dice_roll_outcomes: Vec<u8>,
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
            messages: Vec::new(),
            dice_roll_outcomes,
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

    pub fn execute_playing_action(&mut self, user_action: UserAction, player_index: usize) {
        let action = user_action.action;
        let user_specification = user_action.specification;
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
        action.execute(&mut player, user_specification, self);
        self.players.insert(player_index, player);
        if !free_action {
            self.actions_left -= 1;
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
}

#[derive(Serialize, Deserialize)]
struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    starting_player: usize,
    current_player: usize,
    log: Vec<String>,
    played_limited_actions: Vec<String>,
    actions_left: u32,
    round: u32,
    age: u32,
    messages: Vec<String>,
    dice_roll_outcomes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub enum GameState {
    Playing,
    StatusPhase(StatusPhaseState),
    Finished,
}

#[derive(Serialize, Deserialize)]
pub enum StatusPhaseState {
    CompleteObjectives,
    FreeAdvance,
    DrawNewCards,
    RaseSize1City,
    ChangeGovernmentType,
    DetermineFirstPlayer,
}
