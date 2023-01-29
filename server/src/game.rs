use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    city::Building,
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    hexagon::Position,
    player::{Player, PlayerData},
    playing_actions::PlayingAction::*,
    resource_pile::ResourcePile,
    special_advance::SpecialAdvance,
    status_phase_actions::StatusPhaseAction,
    wonder::Wonder,
};

use crate::status_phase_actions::{next_status_phase, player_that_chooses_next_first_player};
use GameState::*;
use StatusPhaseState::*;

const DICE_ROLL_BUFFER: u32 = 200;
const AGES: u32 = 6;

pub struct Game {
    pub state: GameState,
    pub players: Vec<Player>,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub log: Vec<LogItem>,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub actions_left: u32,
    pub round: u32, // starts with 1
    pub age: u32,   // starts with 1
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
            starting_player_index: starting_player,
            current_player_index: starting_player,
            log: Vec::new(),
            played_once_per_turn_actions: Vec::new(),
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
            starting_player_index: data.current_player_index,
            current_player_index: data.current_player_index,
            actions_left: data.actions_left,
            log: data.log,
            played_once_per_turn_actions: data.played_once_per_turn_actions,
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
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            log: self.log,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
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
        if let Custom(action) = &action {
            let action = action.custom_action_type();
            if self.played_once_per_turn_actions.contains(&action) {
                panic!("Illegal action");
            }
            if action.action_type().once_per_turn {
                self.played_once_per_turn_actions.push(action);
            }
        }
        action.execute(self, player_index);
        if !free_action {
            self.actions_left -= 1;
        }
        if self.actions_left > 0 {
            return;
        }
        if self.players[player_index].resources() != &ResourcePile::empty() {
            return;
        }
        if self
            .get_available_custom_actions()
            .iter()
            .any(|action| action.action_type().free)
        {
            return;
        }
        self.next_turn()
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
        action.execute(self, player_index);
        if matches!(phase, DetermineFirstPlayer) {
            self.next_age();
            return;
        }
        self.next_player();
        if self.current_player_index == self.starting_player_index {
            let next_phase = next_status_phase(phase);
            match next_phase {
                ChangeGovernmentType => {
                    // todo! draw cards (this is a phase of it's on in the rules,
                    // before change government, but execute it right away without storing
                    // the status phase
                }
                DetermineFirstPlayer => {
                    self.current_player_index = player_that_chooses_next_first_player(
                        &self.players,
                        self.starting_player_index,
                    );
                }
                _ => {}
            }

            self.state = StatusPhase(next_phase)
        }
        self.skip_dropped_players();
    }

    fn next_player(&mut self) {
        self.current_player_index += 1;
        self.current_player_index %= self.players.len();
    }

    fn skip_dropped_players(&mut self) {
        while self.dropped_players.contains(&self.current_player_index) {
            self.next_player();
        }
    }

    fn next_turn(&mut self) {
        self.actions_left = 3;
        self.played_once_per_turn_actions = Vec::new();
        self.next_player();
        if self.current_player_index == self.starting_player_index {
            self.next_round();
        }
        self.skip_dropped_players();
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
        self.state = StatusPhase(CompleteObjectives);
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
        self.skip_dropped_players();
    }

    pub fn get_available_custom_actions(&self) -> Vec<CustomActionType> {
        let custom_actions = &self.players[self.current_player_index].custom_actions;
        custom_actions
            .iter()
            .filter(|&action| !self.played_once_per_turn_actions.contains(action))
            .cloned()
            .collect()
    }

    pub fn draw_wonder_card(&mut self, player_index: usize) {
        let wonder = match self.wonders_left.pop() {
            Some(wonder) => wonder,
            None => return,
        };
        self.wonder_amount_left -= 1;
        self.players[player_index].wonder_cards.push(wonder);
    }

    pub fn kill_leader(&mut self, player_index: usize) {
        if let Some(leader) = self.players[player_index].active_leader.take() {
            (leader.player_deinitializer)(self, player_index);
        }
    }

    pub fn set_active_leader(&mut self, leader_index: usize, player_index: usize) {
        self.kill_leader(player_index);
        let new_leader = self.players[player_index]
            .available_leaders
            .remove(leader_index);
        (new_leader.player_initializer)(self, player_index);
        (new_leader.player_one_time_initializer)(self, leader_index);
        self.players[player_index].active_leader = Some(new_leader);
    }

    pub fn advance(&mut self, advance: &str, player_index: usize) {
        let advance = advances::get_advance_by_name(advance).expect("advance should exist");
        (advance.player_initializer)(self, player_index);
        (advance.player_one_time_initializer)(self, player_index);
        for i in 0..self.players[player_index]
            .civilization
            .special_advances
            .len()
        {
            if self.players[player_index].civilization.special_advances[i].required_advance
                == advance.name
            {
                let special_advance = self.players[player_index]
                    .civilization
                    .special_advances
                    .remove(i);
                self.unlock_special_advance(&special_advance, player_index);
                self.players[player_index]
                    .civilization
                    .special_advances
                    .insert(i, special_advance);
                break;
            }
        }
        let player = &mut self.players[player_index];
        if let Some(advance_bonus) = &advance.advance_bonus {
            player.gain_resources(advance_bonus.resources());
        }
        player.advances.push(advance.name);
        player.game_event_tokens -= 1;
        if player.game_event_tokens == 0 {
            player.game_event_tokens = 3;
            self.trigger_game_event(player_index);
        }
    }

    fn trigger_game_event(&mut self, player_index: usize) {
        todo!()
    }

    pub fn remove_advance(&mut self, advance: &str, player_index: usize) {
        if let Some(position) = self.players[player_index]
            .advances
            .iter()
            .position(|other_advance| other_advance == advance)
        {
            let advance = advances::get_advance_by_name(advance).expect("advance should exist");
            (advance.player_deinitializer)(self, player_index);
            self.players[player_index].advances.remove(position);
        }
    }

    fn unlock_special_advance(&mut self, special_advance: &SpecialAdvance, player_index: usize) {
        (special_advance.player_initializer)(self, player_index);
        (special_advance.player_one_time_initializer)(self, player_index);
        self.players[player_index]
            .unlocked_special_advances
            .push(special_advance.name.clone());
    }

    pub fn conquer_city(
        &mut self,
        position: &Position,
        new_player_index: usize,
        old_player_index: usize,
    ) {
        self.players[old_player_index]
            .take_city(position)
            .expect("player should own city")
            .conquer(self, new_player_index, old_player_index);
    }

    pub fn raze_city(&mut self, position: &Position, player_index: usize) {
        let city = self.players[player_index]
            .take_city(position)
            .expect("player should have this city");
        city.raze(self, player_index);
    }

    pub fn build_wonder(&mut self, wonder: Wonder, city: &Position, player_index: usize) {
        let mut wonder = wonder;
        (wonder.player_initializer)(self, player_index);
        (wonder.player_one_time_initializer)(self, player_index);
        wonder.builder = Some(player_index);
        let player = &mut self.players[player_index];
        player.wonders_build += 1;
        player.wonders.push(wonder.name.clone());
        player
            .get_city_mut(city)
            .expect("player should have city")
            .city_pieces
            .wonders
            .push(wonder);
    }

    //this function assumes action is legal
    pub fn influence_culture(
        &mut self,
        influencer_index: usize,
        influenced_player_index: usize,
        city_position: &Position,
        building: &Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced should have influenced city")
            .city_pieces
            .set_building(building, influencer_index);
        self.players[influencer_index].influenced_buildings += 1;
    }
}

#[derive(Serialize, Deserialize)]
struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    starting_player_index: usize,
    current_player_index: usize,
    log: Vec<LogItem>,
    played_once_per_turn_actions: Vec<CustomActionType>,
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
            starting_player_index: 0,
            current_player_index: 0,
            log: Vec::new(),
            played_once_per_turn_actions: Vec::new(),
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
