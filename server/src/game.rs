use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    content::{civilizations, wonders},
    player::{Player, PlayerData},
    playing_actions::PlayingAction::*,
    status_phase_actions::StatusPhaseAction,
    wonder::Wonder,
};

use crate::status_phase_actions::player_that_chooses_next_first_player;
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
    pub wonders_left: Vec<Wonder>,
    pub wonder_amount_left: usize,
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
        let mut rng = StdRng::from_seed(seed);

        let mut players = Vec::new();
        let mut civilizations = civilizations::get_civilizations();
        for i in 0..player_amount {
            let civilization = rng.gen_range(0..civilizations.len());
            players.push(Player::new(civilizations.remove(civilization), i));
        }

        let starting_player = rng.gen_range(0..players.len());
        let mut dice_roll_outcomes = Vec::new();
        for _ in 0..DICE_ROLL_BUFFER {
            dice_roll_outcomes.push(rng.gen_range(1..=12));
        }

        let mut wonders = wonders::get_wonders();
        wonders.shuffle(&mut rng);
        let wonder_amount = wonders.len();

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
            wonders_left: wonders,
            wonder_amount_left: wonder_amount,
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
        let mut game = Self {
            state: data.state,
            players: Vec::new(),
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
            wonders_left: data
                .wonders_left
                .into_iter()
                .map(|wonder| {
                    wonders::get_wonder_by_name(&wonder)
                        .expect("wonder data should have valid wonder names")
                })
                .collect(),
            wonder_amount_left: data.wonder_amount_left,
        };
        let mut players = Vec::new();
        for player in data.players.into_iter() {
            players.push(Player::from_data(player, &mut game));
        }
        game.players = players;
        game
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
            wonders_left: self
                .wonders_left
                .into_iter()
                .map(|wonder| wonder.name)
                .collect(),
            wonder_amount_left: self.wonder_amount_left,
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

    pub fn with_player<F>(&mut self, player_index: usize, action: F)
    where
        F: FnOnce(&mut Player, &mut Game),
    {
        let mut player = self.players.remove(player_index);
        action(&mut player, self);
        self.players.insert(player_index, player);
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
        self.with_player(player_index, |p, g| action.execute(g, p.id));
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

        self.with_player(player_index, |p, g| action.execute(p, g));
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
                    self.current_player =
                        player_that_chooses_next_first_player(&self.players, self.starting_player);
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

    pub fn get_available_custom_actions(&self) -> Vec<String> {
        let custom_actions = &self.players[self.current_player].custom_actions;
        custom_actions
            .iter()
            .filter(|&action| !self.played_limited_actions.contains(action))
            .cloned()
            .collect()
    }

    pub fn draw_wonder_card(&mut self, player: usize) {
        let wonder = match self.wonders_left.pop() {
            Some(wonder) => wonder,
            None => return,
        };
        self.wonder_amount_left -= 1;
        self.players[player].wonder_cards.push(wonder);
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
    wonders_left: Vec<String>,
    wonder_amount_left: usize,
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

#[cfg(test)]
pub mod tests {
    use super::Game;
    use super::GameState::Playing;

    pub fn test_game() -> Game {
        Game {
            state: Playing,
            players: Vec::new(),
            starting_player: 0,
            current_player: 0,
            log: Vec::new(),
            played_limited_actions: Vec::new(),
            actions_left: 3,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            dice_roll_outcomes: vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
            dropped_players: Vec::new(),
            wonders_left: Vec::new(),
            wonder_amount_left: 0,
        }
    }
}
