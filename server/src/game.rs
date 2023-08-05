use std::collections::HashMap;

use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::{
    city_pieces::Building,
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    map::{Map, MapData},
    player::{Player, PlayerData},
    position::Position,
    resource_pile::ResourcePile,
    special_advance::SpecialAdvance,
    status_phase::{
        StatusPhaseAction,
        StatusPhaseState::{self, *},
    },
    wonder::Wonder,
};

use crate::playing_actions::PlayingAction;
use GameState::*;

const DICE_ROLL_BUFFER: u32 = 200;
const AGES: u32 = 6;

pub struct Game {
    pub state: GameState,
    pub players: Vec<Player>,
    pub map: Map,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub log: Vec<LogItem>,
    pub log_index: usize,
    pub undo_limit: usize,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    pub round: u32,
    // starts with 1
    pub age: u32,
    // starts with 1
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

        let map = HashMap::new();
        //todo! generate map

        Self {
            state: Playing,
            players,
            map: Map::new(map),
            starting_player_index: starting_player,
            current_player_index: starting_player,
            log: Vec::new(),
            log_index: 0,
            undo_limit: 0,
            played_once_per_turn_actions: Vec::new(),
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            dice_roll_outcomes,
            dropped_players: Vec::new(),
            wonders_left: wonders,
            wonder_amount_left: wonder_amount,
        }
    }

    pub fn from_data(data: GameData) -> Self {
        let mut game = Self {
            state: data.state,
            players: Vec::new(),
            map: Map::from_data(data.map),
            starting_player_index: data.current_player_index,
            current_player_index: data.current_player_index,
            actions_left: data.actions_left,
            successful_cultural_influence: data.successful_cultural_influence,
            log: data.log,
            log_index: data.log_index,
            undo_limit: data.undo_limit,
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

    pub fn data(self) -> GameData {
        GameData {
            state: self.state,
            players: self
                .players
                .into_iter()
                .map(|player| player.data())
                .collect(),
            map: self.map.data(),
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            log: self.log,
            log_index: self.log_index,
            undo_limit: self.undo_limit,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
            actions_left: self.actions_left,
            successful_cultural_influence: self.successful_cultural_influence,
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

    fn add_log_item(&mut self, item: LogItem) {
        if self.log_index < self.log.len() {
            self.log.drain(self.log_index..);
        }
        self.log.push(item);
        self.log_index += 1;
    }

    pub fn lock_undo(&mut self) {
        self.undo_limit = self.log_index;
    }

    pub fn execute_action(&mut self, action: Action, player_index: usize) {
        if player_index != self.current_player_index {
            panic!("Illegal action");
        }
        match self.state.clone() {
            StatusPhase(phase) => {
                let action = action
                    .as_status_phase_action()
                    .expect("action should be a status phase action");
                self.add_log_item(LogItem::StatusPhaseAction(
                    serde_json::to_string(&action)
                        .expect("status phase action should be serializable"),
                ));
                if phase != action.phase {
                    panic!("Illegal action");
                }
                action.execute(self, player_index);
            }
            CulturalInfluenceResolution {
                roll_boost_cost,
                target_player_index,
                target_city_position,
                city_piece,
            } => {
                let action = action
                    .as_cultural_influence_resolution_action()
                    .expect("action should be a cultural influence resolution action");
                self.add_log_item(LogItem::CulturalInfluenceResolutionAction(
                    serde_json::to_string(&action).expect("playing action should be serializable"),
                ));
                self.execute_cultural_influence_resolution_action(
                    action,
                    roll_boost_cost,
                    target_player_index,
                    target_city_position,
                    city_piece,
                    player_index,
                );
            }
            Playing => {
                if let Action::Undo = action {
                    if !self.can_undo() {
                        panic!("actions revealing new information can't be undone");
                    }
                    self.log_index -= 1;
                    let action = self.log[self.log_index]
                        .as_playing_action()
                        .expect("previous action should be a playing action");
                    let action = serde_json::from_str::<PlayingAction>(action)
                        .expect("action should be deserializable");
                    action.undo(self, player_index);
                    return;
                }
                if let Action::Redo = action {
                    if !self.can_redo() {
                        panic!("no action can be redone");
                    }
                    let action = self.log[self.log_index]
                        .as_playing_action()
                        .expect("undone actions should be playing actions");
                    let action = serde_json::from_str::<PlayingAction>(action)
                        .expect("action should be deserializable");
                    action.execute(self, player_index);
                    self.log_index += 1;
                    return;
                }
                let action = action
                    .as_playing_action()
                    .expect("action should be a playing action");
                if self.can_redo()
                    && serde_json::from_str::<PlayingAction>(
                        self.log[self.log_index]
                            .as_playing_action()
                            .expect("undone actions should be playing actions"),
                    )
                    .expect("action should be deserializable")
                        == action
                {
                    self.log_index += 1;
                    action.execute(self, player_index);
                    return;
                }
                self.add_log_item(LogItem::PlayingAction(
                    serde_json::to_string(&action).expect("playing action should be serializable"),
                ));
                action.execute(self, player_index);
            }
            Finished => panic!("actions can't be executed when the game is finished"),
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo_limit < self.log_index
    }

    pub fn can_redo(&self) -> bool {
        self.log_index < self.log.len()
    }

    fn execute_cultural_influence_resolution_action(
        &mut self,
        action: bool,
        roll_boost_cost: u32,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
        player_index: usize,
    ) {
        self.state = Playing;
        if !action {
            return;
        }
        self.players[player_index].loose_resources(ResourcePile::culture_tokens(roll_boost_cost));
        self.influence_culture(
            player_index,
            target_player_index,
            &target_city_position,
            &city_piece,
        )
    }

    pub fn next_player(&mut self) {
        self.current_player_index += 1;
        self.current_player_index %= self.players.len();
        self.lock_undo();
    }

    pub fn skip_dropped_players(&mut self) {
        while self.dropped_players.contains(&self.current_player_index) {
            self.next_player();
        }
    }

    pub fn next_turn(&mut self) {
        self.actions_left = 3;
        self.successful_cultural_influence = false;
        self.played_once_per_turn_actions = Vec::new();
        for city in self.players[self.current_player_index].cities.iter_mut() {
            city.deactivate();
        }
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

    pub fn next_age(&mut self) {
        self.state = Playing;
        self.age += 1;
        self.lock_undo();
        if self.age > AGES {
            self.end_game();
            return;
        }
        self.add_message(&format!("Age {} has started", self.age));
    }

    fn end_game(&mut self) {
        self.state = Finished;
        self.add_message("Game has ended");
    }

    pub fn get_next_dice_roll(&mut self) -> u8 {
        self.lock_undo();
        self.dice_roll_outcomes.pop().unwrap_or_else(|| {
            println!("ran out of predetermined dice roll outcomes, unseeded rng is no being used");
            rand::thread_rng().gen_range(1..=12)
        })
    }

    fn add_message(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }

    pub fn drop_player(&mut self, player_index: usize) {
        self.dropped_players.push(player_index);
        self.add_message(&format!("Player{} had left the game", player_index + 1));
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
        self.lock_undo()
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
        self.players[player_index].take_events(|events, player| {
            events.on_advance.trigger(player, &advance.to_string(), &())
        });
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

    pub fn undo_advance(&mut self, advance: &str, player_index: usize) {
        self.players[player_index].take_events(|events, player| {
            events
                .on_undo_advance
                .trigger(player, &advance.to_string(), &())
        });
        let advance = advances::get_advance_by_name(advance).expect("advance should exist");
        (advance.player_deinitializer)(self, player_index);
        (advance.player_undo_deinitializer)(self, player_index);
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
                self.undo_unlock_special_advance(&special_advance, player_index);
                self.players[player_index]
                    .civilization
                    .special_advances
                    .insert(i, special_advance);
                break;
            }
        }
        let player = &mut self.players[player_index];
        if let Some(advance_bonus) = &advance.advance_bonus {
            player.loose_resources(advance_bonus.resources());
        }
        player.advances.pop();
        player.game_event_tokens += 1;
    }

    fn trigger_game_event(&mut self, _player_index: usize) {
        self.lock_undo();
        //todo!
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

    fn undo_unlock_special_advance(
        &mut self,
        special_advance: &SpecialAdvance,
        player_index: usize,
    ) {
        (special_advance.player_deinitializer)(self, player_index);
        (special_advance.player_undo_deinitializer)(self, player_index);
        self.players[player_index].unlocked_special_advances.pop();
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

    pub fn build_wonder(&mut self, wonder: Wonder, city_position: &Position, player_index: usize) {
        self.players[player_index].take_events(|events, player| {
            events
                .on_construct_wonder
                .trigger(player, city_position, &wonder)
        });
        let mut wonder = wonder;
        (wonder.player_initializer)(self, player_index);
        (wonder.player_one_time_initializer)(self, player_index);
        wonder.builder = Some(player_index);
        let player = &mut self.players[player_index];
        player.wonders_build += 1;
        player.wonders.push(wonder.name.clone());
        player
            .get_city_mut(city_position)
            .expect("player should have city")
            .city_pieces
            .wonders
            .push(wonder);
    }

    pub fn undo_build_wonder(&mut self, city_position: &Position, player_index: usize) -> Wonder {
        let player = &mut self.players[player_index];
        player.wonders_build -= 1;
        player.wonders.pop();
        let mut wonder = player
            .get_city_mut(city_position)
            .expect("player should have city")
            .city_pieces
            .wonders
            .pop()
            .expect("city should have a wonder");
        self.players[player_index].take_events(|events, player| {
            events
                .on_undo_construct_wonder
                .trigger(player, city_position, &wonder)
        });
        (wonder.player_deinitializer)(self, player_index);
        (wonder.player_undo_deinitializer)(self, player_index);
        wonder.builder = None;
        wonder
    }

    pub fn influence_culture_boost_cost(
        &self,
        player_index: usize,
        starting_city_position: &Position,
        target_player_index: usize,
        target_city_position: &Position,
        city_piece: &Building,
    ) -> Option<ResourcePile> {
        //todo! allow cultural influence of barbarians
        let starting_city = &self.players[player_index]
            .get_city(starting_city_position)
            .expect("player should have position");
        let range_boost = starting_city_position
            .distance(target_city_position)
            .saturating_sub(starting_city.size() as u32);
        let range_boost_cost = ResourcePile::culture_tokens(range_boost);
        let self_influence = starting_city_position == target_city_position;
        let target_city = self.players[target_player_index]
            .get_city(target_city_position)
            .expect("Illegal action");
        let target_city_owner = target_city.player_index;
        let target_building_owner = target_city
            .city_pieces
            .building_owner(city_piece)
            .expect("Illegal action");
        let player = &self.players[player_index];
        let starting_city = player
            .get_city(starting_city_position)
            .expect("Illegal action");
        if matches!(&city_piece, Building::Obelisk)
            || starting_city.player_index != player_index
            || !player.resources().can_afford(&range_boost_cost)
            || (starting_city.influenced() && !self_influence)
            || self.successful_cultural_influence
            || !player.available_buildings.can_build(city_piece)
            || target_city_owner != target_player_index
            || target_building_owner == player_index
        {
            None
        } else {
            Some(range_boost_cost)
        }
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
        self.successful_cultural_influence = true;
        self.players[influenced_player_index].available_buildings += building;
        self.players[influencer_index].available_buildings -= building;
    }

    pub fn draw_new_cards(&mut self) {
        //every player draws 1 action card and 1 objective card
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    log: Vec<LogItem>,
    log_index: usize,
    undo_limit: usize,
    played_once_per_turn_actions: Vec<CustomActionType>,
    actions_left: u32,
    successful_cultural_influence: bool,
    round: u32,
    age: u32,
    messages: Vec<String>,
    dice_roll_outcomes: Vec<u8>,
    dropped_players: Vec<usize>,
    wonders_left: Vec<String>,
    wonder_amount_left: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Playing,
    StatusPhase(StatusPhaseState),
    CulturalInfluenceResolution {
        roll_boost_cost: u32,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
    },
    Finished,
}

#[derive(Serialize, Deserialize)]
pub enum Action {
    PlayingAction(PlayingAction),
    StatusPhaseAction(StatusPhaseAction),
    CulturalInfluenceResolutionAction(bool),
    Undo,
    Redo,
}

impl Action {
    pub fn as_playing_action(self) -> Option<PlayingAction> {
        if let Self::PlayingAction(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_status_phase_action(self) -> Option<StatusPhaseAction> {
        if let Self::StatusPhaseAction(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_cultural_influence_resolution_action(self) -> Option<bool> {
        if let Self::CulturalInfluenceResolutionAction(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum LogItem {
    PlayingAction(String),
    StatusPhaseAction(String),
    CulturalInfluenceResolutionAction(String),
}

impl LogItem {
    pub fn as_playing_action(&self) -> Option<&str> {
        if let Self::PlayingAction(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use crate::{
        city::{City, MoodState::*},
        city_pieces::Building::*,
        content::civilizations,
        map::Map,
        player::Player,
        position::Position,
        resource_pile::ResourcePile,
        wonder::Wonder,
    };

    use super::{Game, GameState::Playing};

    pub fn test_game() -> Game {
        Game {
            state: Playing,
            players: Vec::new(),
            map: Map::new(HashMap::new()),
            starting_player_index: 0,
            current_player_index: 0,
            log: Vec::new(),
            log_index: 0,
            undo_limit: 0,
            played_once_per_turn_actions: Vec::new(),
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            dice_roll_outcomes: vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
            dropped_players: Vec::new(),
            wonders_left: Vec::new(),
            wonder_amount_left: 0,
        }
    }

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", ResourcePile::empty(), vec![]).build();
        let mut game = test_game();
        game.players.push(old);
        game.players.push(new);
        let old = 0;
        let new = 1;

        let position = Position::new(0, 0);
        game.players[old]
            .cities
            .push(City::new(old, position.clone()));
        game.build_wonder(wonder, &position, old);
        game.players[old].construct(&Academy, &position);
        game.players[old].construct(&Obelisk, &position);

        assert_eq!(7.0, game.players[old].victory_points());

        game.conquer_city(&position, new, old);

        let c = game.players[new].get_city_mut(&position).unwrap();
        assert_eq!(1, c.player_index);
        assert_eq!(Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        assert_eq!(3.0, old.victory_points());
        assert_eq!(4.0, new.victory_points());
        assert_eq!(0, old.wonders.len());
        assert_eq!(1, new.wonders.len());
        assert_eq!(1, old.influenced_buildings);
        assert_eq!(0, new.influenced_buildings);
    }
}
