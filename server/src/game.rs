use std::collections::HashMap;
use std::mem;

use serde::{Deserialize, Serialize};

use crate::combat::Combat;
use crate::combat::{capture_position, execute_combat_action, initiate_combat, CombatPhase};
use crate::map::{maximum_size_2_player_random_map, setup_home_city};
use crate::utils::shuffle;
use crate::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::Building::{self, *},
    consts::{AGES, DICE_ROLL_BUFFER},
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    log::{self, ActionLogItem},
    map::{Map, MapData, Terrain::*},
    player::{Player, PlayerData},
    playing_actions::PlayingAction,
    position::Position,
    resource_pile::ResourcePile,
    special_advance::SpecialAdvance,
    status_phase::{
        self,
        StatusPhaseState::{self},
    },
    unit::{
        MovementAction::{self, *},
        Unit,
        UnitType::{self, *},
    },
    utils,
    wonder::Wonder,
};
use GameState::*;

pub struct Game {
    pub state: GameState,
    pub players: Vec<Player>,
    pub map: Map,
    pub starting_player_index: usize,
    current_player_index: usize,
    pub action_log: Vec<ActionLogItem>,
    pub action_log_index: usize,
    pub log: Vec<String>,
    pub undo_limit: usize,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    pub round: u32, // starts with 1
    pub age: u32,   // starts with 1
    pub messages: Vec<String>,
    pub dice_roll_outcomes: Vec<u8>,
    pub dice_roll_log: Vec<u8>,
    pub dropped_players: Vec<usize>,
    pub wonders_left: Vec<Wonder>,
    pub wonder_amount_left: usize,
    pub undo_context_stack: Vec<UndoContext>,
}

impl Clone for Game {
    fn clone(&self) -> Self {
        Self::from_data(self.cloned_data())
    }
}

impl Game {
    /// Creates a new [`Game`].
    ///
    /// # Panics
    ///
    /// Panics only if there is an internal bug
    #[must_use]
    pub fn new(player_amount: usize, seed: String, setup: bool) -> Self {
        let seed_length = seed.len();
        let seed = if seed_length < 32 {
            seed + &" ".repeat(32 - seed_length)
        } else {
            String::from(&seed[..32])
        };
        let s: &[u8] = seed.as_bytes();
        let mut buf = [0u8; 8];
        let len = 8.min(s.len());
        buf[..len].copy_from_slice(&s[..len]);
        let seed = u64::from_be_bytes(buf);
        quad_rand::srand(seed);

        let mut players = Vec::new();
        let mut civilizations = civilizations::get_all();
        for i in 0..player_amount {
            let civilization = quad_rand::gen_range(0, civilizations.len());
            players.push(Player::new(civilizations.remove(civilization), i));
        }

        if setup {
            setup_home_city(&mut players, 0, "F1");
            setup_home_city(&mut players, 1, "F8");
        }
        let starting_player = quad_rand::gen_range(0, players.len());
        let mut dice_roll_outcomes = Vec::new();
        for _ in 0..DICE_ROLL_BUFFER {
            dice_roll_outcomes.push(quad_rand::gen_range(0, 12));
        }

        let wonders = shuffle(&mut wonders::get_all());
        let wonder_amount = wonders.len();

        let map = if setup {
            Map::new(maximum_size_2_player_random_map())
        } else {
            Map::new(HashMap::new())
        };
        Self {
            state: Playing,
            players,
            map,
            starting_player_index: starting_player,
            current_player_index: starting_player,
            action_log: Vec::new(),
            action_log_index: 0,
            log: vec![
                String::from("The game has started"),
                String::from("Age 1 has started"),
                String::from("Round 1/3"),
            ],
            undo_limit: 0,
            played_once_per_turn_actions: Vec::new(),
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("The game has started")],
            dice_roll_outcomes,
            dice_roll_log: Vec::new(),
            dropped_players: Vec::new(),
            wonders_left: wonders,
            wonder_amount_left: wonder_amount,
            undo_context_stack: Vec::new(),
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if any wonder does not exist
    #[must_use]
    pub fn from_data(data: GameData) -> Self {
        let mut game = Self {
            state: data.state,
            players: Vec::new(),
            map: Map::from_data(data.map),
            starting_player_index: data.starting_player_index,
            current_player_index: data.current_player_index,
            actions_left: data.actions_left,
            successful_cultural_influence: data.successful_cultural_influence,
            action_log: data.action_log,
            action_log_index: data.action_log_index,
            log: data.log,
            undo_limit: data.undo_limit,
            played_once_per_turn_actions: data.played_once_per_turn_actions,
            round: data.round,
            age: data.age,
            messages: data.messages,
            dice_roll_outcomes: data.dice_roll_outcomes,
            dice_roll_log: data.dice_roll_log,
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
            undo_context_stack: data.undo_context_stack,
        };
        for player in data.players {
            Player::initialize_player(player, &mut game);
        }
        game
    }

    #[must_use]
    pub fn data(self) -> GameData {
        GameData {
            state: self.state,
            players: self.players.into_iter().map(Player::data).collect(),
            map: self.map.data(),
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            action_log: self.action_log,
            action_log_index: self.action_log_index,
            log: self.log,
            undo_limit: self.undo_limit,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
            actions_left: self.actions_left,
            successful_cultural_influence: self.successful_cultural_influence,
            round: self.round,
            age: self.age,
            messages: self.messages,
            dice_roll_outcomes: self.dice_roll_outcomes,
            dice_roll_log: self.dice_roll_log,
            dropped_players: self.dropped_players,
            wonders_left: self
                .wonders_left
                .into_iter()
                .map(|wonder| wonder.name)
                .collect(),
            wonder_amount_left: self.wonder_amount_left,
            undo_context_stack: self.undo_context_stack,
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> GameData {
        GameData {
            state: self.state.clone(),
            players: self.players.iter().map(Player::cloned_data).collect(),
            map: self.map.cloned_data(),
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            action_log: self.action_log.clone(),
            action_log_index: self.action_log_index,
            log: self.log.clone(),
            undo_limit: self.undo_limit,
            played_once_per_turn_actions: self.played_once_per_turn_actions.clone(),
            actions_left: self.actions_left,
            successful_cultural_influence: self.successful_cultural_influence,
            round: self.round,
            age: self.age,
            messages: self.messages.clone(),
            dice_roll_outcomes: self.dice_roll_outcomes.clone(),
            dice_roll_log: self.dice_roll_log.clone(),
            dropped_players: self.dropped_players.clone(),
            wonders_left: self
                .wonders_left
                .iter()
                .map(|wonder| wonder.name.clone())
                .collect(),
            wonder_amount_left: self.wonder_amount_left,
            undo_context_stack: self.undo_context_stack.clone(),
        }
    }

    #[must_use]
    pub fn get_player(&self, player_index: usize) -> &Player {
        &self.players[player_index]
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the city does not exist
    ///
    /// if you want to get an option instead, use `Player::get_city` function
    #[must_use]
    pub fn get_city(&self, player_index: usize, position: Position) -> &City {
        self.get_player(player_index)
            .get_city(position)
            .expect("city not found")
    }

    #[must_use]
    pub fn get_any_city(&self, position: Position) -> Option<&City> {
        self.players
            .iter()
            .find_map(|player| player.get_city(position))
    }

    fn add_action_log_item(&mut self, item: ActionLogItem) {
        if self.action_log_index < self.action_log.len() {
            self.action_log.drain(self.action_log_index..);
        }
        self.action_log.push(item);
        self.action_log_index += 1;
    }

    pub fn add_info_log_item(&mut self, info: String) {
        self.log.push(info);
        self.lock_undo();
    }

    pub fn lock_undo(&mut self) {
        self.undo_limit = self.action_log_index;
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the action is illegal
    pub fn execute_action(&mut self, action: Action, player_index: usize) {
        assert!(player_index == self.active_player(), "Illegal action");
        if let Action::Undo = action {
            assert!(
                self.can_undo(),
                "actions revealing new information can't be undone"
            );
            self.undo(player_index);
            return;
        }
        if let Action::Redo = action {
            assert!(self.can_redo(), "no action can be redone");
            self.redo(player_index);
            return;
        }
        self.log.push(log::format_action_log_item(&action, self));
        match self.state.clone() {
            Playing => {
                let action = action.playing().expect("action should be a playing action");
                if self.can_redo()
                    && self.action_log[self.action_log_index]
                        .as_playing_action()
                        .expect("undone actions should be playing actions")
                        == action
                {
                    self.redo(player_index);
                    return;
                }
                self.add_action_log_item(ActionLogItem::Playing(action.clone()));
                action.execute(self, player_index);
            }
            StatusPhase(phase) => {
                let action = action
                    .status_phase()
                    .expect("action should be a status phase action");
                assert!(phase == action.phase(), "Illegal action");
                self.add_action_log_item(ActionLogItem::StatusPhase(action.clone()));
                action.execute(self, player_index);
            }
            Movement {
                movement_actions_left,
                moved_units,
            } => {
                let action = action
                    .movement()
                    .expect("action should be a movement action");
                self.add_action_log_item(ActionLogItem::Movement(action.clone()));
                self.execute_movement_action(
                    action,
                    player_index,
                    movement_actions_left,
                    moved_units,
                );
            }
            CulturalInfluenceResolution(c) => {
                let action = action
                    .cultural_influence_resolution()
                    .expect("action should be a cultural influence resolution action");
                self.add_action_log_item(ActionLogItem::CulturalInfluenceResolution(action));
                self.execute_cultural_influence_resolution_action(
                    action,
                    c.roll_boost_cost,
                    c.target_player_index,
                    c.target_city_position,
                    &c.city_piece,
                    player_index,
                );
            }
            Combat(c) => {
                let action = action.combat().expect("action should be a combat action");
                self.add_action_log_item(ActionLogItem::Combat(action.clone()));
                execute_combat_action(self, action, c);
            }
            PlaceSettler {
                player_index,
                movement_actions_left,
                moved_units,
            } => {
                let action = action
                    .place_settler()
                    .expect("action should be place_settler action");
                self.add_action_log_item(ActionLogItem::PlaceSettler(action));
                self.execute_place_settler_action(
                    action,
                    player_index,
                    movement_actions_left,
                    moved_units,
                );
            }
            Finished => panic!("actions can't be executed when the game is finished"),
        }
    }

    fn undo(&mut self, player_index: usize) {
        match &self.action_log[self.action_log_index - 1] {
            ActionLogItem::Playing(action) => action.clone().undo(self, player_index),
            ActionLogItem::StatusPhase(_) => panic!("status phase actions can't be undone"),
            ActionLogItem::Movement(action) => {
                self.undo_movement_action(action.clone(), player_index);
            }
            ActionLogItem::CulturalInfluenceResolution(action) => {
                self.undo_cultural_influence_resolution_action(*action);
            }
            ActionLogItem::Combat(_action) => unimplemented!("retreat can't yet be undone"),
            ActionLogItem::PlaceSettler(_action) => panic!("placing a settler can't be undone"),
        }
        self.action_log_index -= 1;
        self.log.remove(self.log.len() - 1);
    }

    fn redo(&mut self, player_index: usize) {
        let action_log_item = &self.action_log[self.action_log_index];
        self.log.push(log::format_action_log_item(
            &action_log_item.clone().as_action(),
            self,
        ));
        match action_log_item {
            ActionLogItem::Playing(action) => action.clone().execute(self, player_index),
            ActionLogItem::StatusPhase(_) => panic!("status phase actions can't be redone"),
            ActionLogItem::Movement(action) => {
                let Movement {
                    movement_actions_left,
                    moved_units,
                } = &self.state
                else {
                    panic!(
                        "movement actions can only be redone if the game is in a movement state"
                    );
                };
                self.execute_movement_action(
                    action.clone(),
                    player_index,
                    *movement_actions_left,
                    moved_units.clone(),
                );
            }
            ActionLogItem::CulturalInfluenceResolution(action) => {
                let CulturalInfluenceResolution(c) = &self.state else {
                    panic!("cultural influence resolution actions can only be redone if the game is in a cultural influence resolution state");
                };
                self.execute_cultural_influence_resolution_action(
                    *action,
                    c.roll_boost_cost,
                    c.target_player_index,
                    c.target_city_position,
                    &c.city_piece.clone(),
                    player_index,
                );
            }
            ActionLogItem::Combat(_) => unimplemented!("retreat can't yet be redone"),
            ActionLogItem::PlaceSettler(_) => panic!("place settler actions can't be redone"),
        }
        self.action_log_index += 1;
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.undo_limit < self.action_log_index
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.action_log_index < self.action_log.len()
    }

    fn execute_movement_action(
        &mut self,
        action: MovementAction,
        player_index: usize,
        movement_actions_left: u32,
        mut moved_units: Vec<u32>,
    ) {
        let starting_position = match action {
            Move { units, destination } => {
                let player = &self.players[player_index];
                let starting_position = player
                    .get_unit(*units.first().expect(
                        "instead of providing no units to move a stop movement actions should be done",
                    ))
                    .expect("the player should have all units to move")
                    .position;
                player
                    .can_move_units(
                        self,
                        &units,
                        starting_position,
                        destination,
                        movement_actions_left,
                        &moved_units,
                    )
                    .expect("Illegal action");
                moved_units.extend(units.iter());
                if let Some(defender) = self.enemy_player(player_index, destination) {
                    if self.players[defender]
                        .get_units(destination)
                        .iter()
                        .any(|unit| !unit.unit_type.is_settler())
                    {
                        let mut military = false;
                        for unit_id in &units {
                            let unit = self.players[player_index]
                                .get_unit_mut(*unit_id)
                                .expect("the player should have all units to move");
                            assert!(unit.can_attack());
                            if !unit.unit_type.is_settler() {
                                military = true;
                            }
                            self.move_unit(player_index, *unit_id, destination);
                            self.players[player_index]
                                .get_unit_mut(*unit_id)
                                .expect("the player should have all units to move")
                                .position = starting_position;
                        }
                        assert!(military, "Illegal action");
                        self.state = if movement_actions_left > 1 {
                            Movement {
                                movement_actions_left: movement_actions_left - 1,
                                moved_units: moved_units.clone(),
                            }
                        } else {
                            Playing
                        };

                        initiate_combat(
                            self,
                            defender,
                            destination,
                            player_index,
                            starting_position,
                            units,
                            true,
                            None,
                        );
                        if matches!(self.state, Combat(_)) {
                            return;
                        }
                    }
                } else {
                    self.move_units(player_index, &units, destination);
                }
                if !self.players[player_index].get_units(destination).is_empty() {
                    //todo this should be inside combat_loop, so the conquer can be done later, too
                    for enemy in 0..self.players.len() {
                        if enemy == player_index {
                            continue;
                        }
                        capture_position(self, enemy, destination, player_index);
                    }
                }
                self.state = if movement_actions_left > 1 {
                    Movement {
                        movement_actions_left: movement_actions_left - 1,
                        moved_units: moved_units.clone(),
                    }
                } else {
                    Playing
                };
                Some(starting_position)
            }
            Stop => {
                self.state = Playing;
                None
            }
        };
        self.undo_context_stack.push(UndoContext::Movement {
            starting_position,
            movement_actions_left,
            moved_units,
        });
    }

    fn undo_movement_action(&mut self, action: MovementAction, player_index: usize) {
        let Some(UndoContext::Movement {
            starting_position,
            movement_actions_left,
            mut moved_units,
        }) = self.undo_context_stack.pop()
        else {
            panic!("when undoing a movement action, the game should have stored movement context")
        };
        if let Move {
            units,
            destination: _,
        } = action
        {
            if !units.is_empty() {
                moved_units.drain(moved_units.len() - units.len()..);
            }
            self.undo_move_units(
                player_index,
                units,
                starting_position.expect(
                    "undo context should contain the starting position if units where moved",
                ),
            );
        }
        self.state = Movement {
            movement_actions_left,
            moved_units,
        };
    }

    fn execute_cultural_influence_resolution_action(
        &mut self,
        action: bool,
        roll_boost_cost: u32,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: &Building,
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
            target_city_position,
            city_piece,
        );
    }

    fn undo_cultural_influence_resolution_action(&mut self, action: bool) {
        let cultural_influence_attempt_action = self.action_log[self.action_log_index - 2].as_playing_action().expect("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        let PlayingAction::InfluenceCultureAttempt {
            starting_city_position: _,
            target_player_index,
            target_city_position,
            city_piece,
        } = cultural_influence_attempt_action
        else {
            panic!("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        };
        let roll =
            self.dice_roll_log.last().expect(
                "there should be a dice roll before a cultural influence resolution action",
            ) / 2
                + 1;
        let roll_boost_cost = 5 - roll as u32;
        self.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
            roll_boost_cost,
            target_player_index,
            target_city_position,
            city_piece: city_piece.clone(),
        });
        if !action {
            return;
        }
        self.players[self.current_player_index]
            .gain_resources(ResourcePile::culture_tokens(roll_boost_cost));
        self.undo_influence_culture(
            self.current_player_index,
            target_player_index,
            target_city_position,
            &city_piece,
        );
    }

    fn execute_place_settler_action(
        &mut self,
        action: Position,
        player_index: usize,
        movement_actions_left: u32,
        moved_units: Vec<u32>,
    ) {
        let player = &mut self.players[player_index];
        assert!(player.get_city(action).is_some(), "Illegal action");
        player.add_unit(action, Settler);
        self.state = if movement_actions_left == 0 {
            Playing
        } else {
            Movement {
                movement_actions_left,
                moved_units,
            }
        };
    }

    fn enemy_player(&self, player_index: usize, position: Position) -> Option<usize> {
        self.players.iter().position(|player| {
            player.index != player_index && !player.get_units(position).is_empty()
        })
    }

    pub fn add_to_last_log_item(&mut self, edit: &str) {
        let last_item_index = self.log.len() - 1;
        self.log[last_item_index] += edit;
    }

    pub fn next_player(&mut self) {
        self.increment_player_index();
        self.add_info_log_item(format!(
            "It's {}'s turn",
            self.players[self.current_player_index].get_name()
        ));
        self.lock_undo();
    }

    pub fn skip_dropped_players(&mut self) {
        if self.players.is_empty() {
            return;
        }
        while self.dropped_players.contains(&self.current_player_index)
            && self.current_player_index != self.starting_player_index
        {
            self.increment_player_index();
        }
    }

    pub fn increment_player_index(&mut self) {
        self.current_player_index += 1;
        self.current_player_index %= self.players.len();
    }

    #[must_use]
    pub fn active_player(&self) -> usize {
        match &self.state {
            Combat(c) => match c.phase {
                CombatPhase::RemoveCasualties {
                    player,
                    casualties: _,
                    defender_hits: _,
                }
                | CombatPhase::PlayActionCard(player) => player,
                CombatPhase::Retreat => c.attacker,
            },
            PlaceSettler {
                player_index,
                movement_actions_left: _,
                moved_units: _,
            } => *player_index,
            _ => self.current_player_index,
        }
    }

    pub fn next_turn(&mut self) {
        self.actions_left = 3;
        self.successful_cultural_influence = false;
        self.played_once_per_turn_actions = Vec::new();
        self.players[self.current_player_index].end_turn();
        self.next_player();
        self.skip_dropped_players();
        if self.current_player_index == self.starting_player_index {
            self.next_round();
        }
    }

    fn next_round(&mut self) {
        self.round += 1;
        self.skip_dropped_players();
        if self.round > 3 {
            self.round = 1;
            self.enter_status_phase();
            return;
        }
        self.add_info_log_item(format!("Round {}/3", self.round));
    }

    fn enter_status_phase(&mut self) {
        if self.players.iter().any(|player| player.cities.is_empty()) {
            self.end_game();
        }
        self.add_info_log_item(format!(
            "The game has entered the {} status phase",
            utils::ordinal_number(self.age)
        ));
        status_phase::skip_status_phase_players(self);
    }

    pub fn next_age(&mut self) {
        if self.age == 6 {
            self.state = Finished;
            return;
        }
        self.state = Playing;
        self.age += 1;
        self.current_player_index = self.starting_player_index;
        self.lock_undo();
        if self.age > AGES {
            self.end_game();
            return;
        }
        self.add_info_log_item(format!("Age {} has started", self.age));
        self.add_info_log_item(String::from("Round 1/3"));
    }

    fn end_game(&mut self) {
        self.state = Finished;
        let winner_player_index = self
            .players
            .iter()
            .enumerate()
            .max_by(|(_, player), (_, other)| player.compare_score(other))
            .expect("there should be at least one player in the game")
            .0;
        let winner_name = self.players[winner_player_index].get_name();
        self.add_info_log_item(format!("The game has ended\n{winner_name} has won"));
        self.add_message("The game has ended");
    }

    pub fn get_next_dice_roll(&mut self) -> u8 {
        self.lock_undo();
        let dice_roll = self.dice_roll_outcomes.pop().unwrap_or_else(|| {
            println!("ran out of predetermined dice roll outcomes, unseeded rng is now being used");

            quad_rand::gen_range(0, 12)
        });
        self.dice_roll_log.push(dice_roll);
        dice_roll
    }

    fn add_message(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }

    pub fn drop_player(&mut self, player_index: usize) {
        self.dropped_players.push(player_index);
        self.add_message(&format!(
            "{} has left the game",
            self.players[player_index].get_name()
        ));
        if self.current_player_index != player_index {
            return;
        }
        self.skip_dropped_players();
        if self.current_player_index == self.starting_player_index {
            self.next_round();
        }
    }

    #[must_use]
    pub fn get_available_custom_actions(&self) -> Vec<CustomActionType> {
        let custom_actions = &self.players[self.current_player_index].custom_actions;
        custom_actions
            .iter()
            .filter(|&action| !self.played_once_per_turn_actions.contains(action))
            .cloned()
            .collect()
    }

    pub fn draw_wonder_card(&mut self, player_index: usize) {
        let Some(wonder) = self.wonders_left.pop() else {
            return;
        };

        self.wonder_amount_left -= 1;
        self.players[player_index].wonder_cards.push(wonder);
        self.lock_undo();
    }

    pub fn set_active_leader(&mut self, leader_index: usize, player_index: usize) {
        let new_leader = self.players[player_index]
            .available_leaders
            .remove(leader_index);
        (new_leader.player_initializer)(self, player_index);
        (new_leader.player_one_time_initializer)(self, leader_index);
        self.players[player_index].active_leader = Some(new_leader);
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if advance does not exist
    pub fn advance(&mut self, advance: &str, player_index: usize) {
        self.players[player_index].take_events(|events, player| {
            events.on_advance.trigger(player, &advance.to_string(), &());
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

    ///
    ///
    /// # Panics
    ///
    /// Panics if advance does not exist
    pub fn undo_advance(&mut self, advance: &str, player_index: usize) {
        self.players[player_index].take_events(|events, player| {
            events
                .on_undo_advance
                .trigger(player, &advance.to_string(), &());
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

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist or if a ship is build without a port in the city
    ///
    /// this function assumes that the action is legal
    pub fn recruit(
        &mut self,
        player_index: usize,
        units: Vec<UnitType>,
        city_position: Position,
        leader_index: Option<usize>,
        replaced_units: Vec<u32>,
    ) {
        let mut replaced_leader = None;
        if let Some(leader_index) = leader_index {
            if let Some(previous_leader) = self.players[player_index].active_leader.take() {
                (previous_leader.player_deinitializer)(self, player_index);
                replaced_leader = Some(previous_leader.name);
            }
            self.set_active_leader(leader_index, player_index);
        }
        let player = &mut self.players[player_index];
        let mut replaced_units_undo_context = Vec::new();
        for unit in replaced_units {
            let unit = player
                .remove_unit(unit)
                .expect("the player should have the replaced units");
            player.available_units += &unit.unit_type;
            replaced_units_undo_context.push(unit);
        }
        self.undo_context_stack.push(UndoContext::Recruit {
            replaced_units: replaced_units_undo_context,
            replaced_leader,
        });
        let mut ships = Vec::new();
        player.units.reserve_exact(units.len());
        for unit_type in units {
            let city = player
                .get_city(city_position)
                .expect("player should have a city at the recruitment position");
            let position = match &unit_type {
                Ship => {
                    ships.push(player.next_unit_id);
                    city.port_position
                        .expect("there should be a port in the city")
                }
                _ => city_position,
            };
            player.add_unit(position, unit_type);
        }
        let city = player
            .get_city_mut(city_position)
            .expect("player should have a city at the recruitment position");
        city.activate();
        if !ships.is_empty() {
            let port_position = self.players[player_index]
                .get_city(city_position)
                .and_then(|city| city.port_position)
                .expect("there should be a port");
            if let Some(defender) = self.enemy_player(player_index, port_position) {
                for ship in self.players[player_index].get_units_mut(port_position) {
                    ship.position = city_position;
                }
                initiate_combat(
                    self,
                    defender,
                    port_position,
                    player_index,
                    city_position,
                    ships,
                    false,
                    Some(Playing),
                );
            }
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn undo_recruit(
        &mut self,
        player_index: usize,
        units: &Vec<UnitType>,
        city_position: Position,
        leader_index: Option<usize>,
    ) {
        if let Some(leader_index) = leader_index {
            let current_leader = self.players[player_index]
                .active_leader
                .take()
                .expect("the player should have an active leader");
            (current_leader.player_deinitializer)(self, player_index);
            (current_leader.player_undo_deinitializer)(self, player_index);
            self.players[player_index]
                .available_leaders
                .insert(leader_index, current_leader);
            self.players[player_index].active_leader = None;
        }
        let player = &mut self.players[player_index];
        for _ in 0..units.len() {
            let unit = player
                .units
                .pop()
                .expect("the player should have the recruited units when undoing");
            player.available_units += &unit.unit_type;
            player.next_unit_id -= 1;
        }
        player
            .get_city_mut(city_position)
            .expect("player should have a city a recruitment position")
            .undo_activate();
        if let Some(UndoContext::Recruit {
            replaced_units,
            replaced_leader,
        }) = self.undo_context_stack.pop()
        {
            for unit in replaced_units {
                player.available_units -= &unit.unit_type;
                player.units.push(unit);
            }
            if let Some(replaced_leader) = replaced_leader {
                let replaced_leader =
                    civilizations::get_leader_by_name(&replaced_leader, &player.civilization.name)
                        .expect("there should be a replaced leader in context data");
                (replaced_leader.player_initializer)(self, player_index);
                (replaced_leader.player_one_time_initializer)(self, player_index);
                let player = &mut self.players[player_index];
                player.active_leader = Some(replaced_leader);
            }
        }
    }

    fn trigger_game_event(&mut self, _player_index: usize) {
        self.lock_undo();
        //todo
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if advance does not exist
    pub fn remove_advance(&mut self, advance: &str, player_index: usize) {
        utils::remove_element(
            &mut self.players[player_index].advances,
            &advance.to_string(),
        );
        let advance = advances::get_advance_by_name(advance).expect("advance should exist");
        (advance.player_deinitializer)(self, player_index);
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

    ///
    ///
    /// # Panics
    ///
    /// Panics if the city does not exist or if the game is not in a movement state
    pub fn conquer_city(
        &mut self,
        position: Position,
        new_player_index: usize,
        old_player_index: usize,
    ) {
        self.add_to_last_log_item(&format!(
            " and captured {}'s city at {position}",
            self.players[old_player_index].get_name()
        ));
        let Some(mut city) = self.players[old_player_index].take_city(position) else {
            return;
        };
        self.players[new_player_index]
            .gain_resources(ResourcePile::gold(city.mood_modified_size() as i32));
        let settlements_left = self.players[new_player_index].available_settlements > 0;
        if settlements_left {
            for wonder in &city.pieces.wonders {
                (wonder.player_deinitializer)(self, old_player_index);
                (wonder.player_initializer)(self, new_player_index);
            }
        }
        city.player_index = new_player_index;
        city.mood_state = Angry;
        for wonder in &city.pieces.wonders {
            self.players[old_player_index].remove_wonder(wonder);
            if settlements_left {
                self.players[new_player_index]
                    .wonders
                    .push(wonder.name.clone());
            }
        }
        if let Some(player) = &city.pieces.obelisk {
            if player == &old_player_index {
                self.players[old_player_index].influenced_buildings += 1;
            }
        }
        let previously_influenced_building =
            city.pieces.buildings(Some(new_player_index)).len() as u32;
        for (building, owner) in city.pieces.building_owners() {
            if matches!(building, Obelisk) {
                if !settlements_left {
                    self.players[old_player_index].available_buildings += &building;
                    self.players[old_player_index].influenced_buildings -= 1;
                }
                continue;
            }
            let Some(owner) = owner else {
                continue;
            };
            if owner != old_player_index {
                if !settlements_left {
                    self.players[owner].available_buildings += &building;
                    self.players[owner].influenced_buildings -= 1;
                }
                continue;
            }
            city.pieces.set_building(&building, new_player_index);
            self.players[old_player_index].available_buildings += &building;
            if self.players[new_player_index]
                .available_buildings
                .can_build(&building)
            {
                self.players[new_player_index].available_buildings -= &building;
            } else {
                city.pieces.remove_building(&building);
                self.players[new_player_index].gain_resources(ResourcePile::gold(1));
            }
        }
        let new_player = &mut self.players[new_player_index];
        new_player.influenced_buildings -= previously_influenced_building;
        if settlements_left {
            new_player.cities.push(city);
            new_player.available_settlements -= 1;
        } else {
            new_player.gain_resources(ResourcePile::gold(city.size() as i32));
        }
        let old_player = &mut self.players[old_player_index];
        old_player.available_settlements += 1;
        if old_player.available_units.settlers > 0 && !old_player.cities.is_empty() {
            let state = mem::replace(&mut self.state, Playing);
            let Movement {
                movement_actions_left,
                moved_units,
            } = state
            else {
                panic!("conquering a city should only happen in a movement action")
            };
            self.state = PlaceSettler {
                player_index: old_player_index,
                movement_actions_left: movement_actions_left - 1,
                moved_units,
            };
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the city does not exist
    pub fn raze_city(&mut self, position: Position, player_index: usize) {
        let city = self.players[player_index]
            .take_city(position)
            .expect("player should have this city");
        city.raze(self, player_index);
        self.players[player_index].available_settlements += 1;
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn build_wonder(&mut self, wonder: Wonder, city_position: Position, player_index: usize) {
        self.players[player_index].take_events(|events, player| {
            events
                .on_construct_wonder
                .trigger(player, &city_position, &wonder);
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
            .pieces
            .wonders
            .push(wonder);
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if city or wonder does not exist
    pub fn undo_build_wonder(&mut self, city_position: Position, player_index: usize) -> Wonder {
        let player = &mut self.players[player_index];
        player.wonders_build -= 1;
        player.wonders.pop();
        let mut wonder = player
            .get_city_mut(city_position)
            .expect("player should have city")
            .pieces
            .wonders
            .pop()
            .expect("city should have a wonder");
        self.players[player_index].take_events(|events, player| {
            events
                .on_undo_construct_wonder
                .trigger(player, &city_position, &wonder);
        });
        (wonder.player_deinitializer)(self, player_index);
        (wonder.player_undo_deinitializer)(self, player_index);
        wonder.builder = None;
        wonder
    }

    #[must_use]
    pub fn influence_culture_boost_cost(
        &self,
        player_index: usize,
        starting_city_position: Position,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: &Building,
    ) -> Option<ResourcePile> {
        //todo allow cultural influence of barbarians
        let starting_city = self.get_city(player_index, starting_city_position);
        let range_boost = starting_city_position
            .distance(target_city_position)
            .saturating_sub(starting_city.size() as u32);
        let range_boost_cost = ResourcePile::culture_tokens(range_boost);
        let self_influence = starting_city_position == target_city_position;
        let target_city = self.get_city(target_player_index, target_city_position);
        let target_city_owner = target_city.player_index;
        let target_building_owner = target_city.pieces.building_owner(city_piece)?;
        let player = &self.players[player_index];
        if matches!(&city_piece, Building::Obelisk)
            || starting_city.player_index != player_index
            || !player.resources.can_afford(&range_boost_cost)
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

    ///
    ///
    /// # Panics
    ///
    /// Panics if the influenced player does not have the influenced city
    /// This function assumes the action is legal
    pub fn influence_culture(
        &mut self,
        influencer_index: usize,
        influenced_player_index: usize,
        city_position: Position,
        building: &Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced player should have influenced city")
            .pieces
            .set_building(building, influencer_index);
        self.players[influencer_index].influenced_buildings += 1;
        self.successful_cultural_influence = true;
        self.players[influenced_player_index].available_buildings += building;
        self.players[influencer_index].available_buildings -= building;
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the influenced player does not have the influenced city
    pub fn undo_influence_culture(
        &mut self,
        influencer_index: usize,
        influenced_player_index: usize,
        city_position: Position,
        building: &Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced player should have influenced city")
            .pieces
            .set_building(building, influenced_player_index);
        self.players[influencer_index].influenced_buildings -= 1;
        self.successful_cultural_influence = false;
        self.players[influenced_player_index].available_buildings -= building;
        self.players[influencer_index].available_buildings += building;
    }

    pub fn draw_new_cards(&mut self) {
        //todo every player draws 1 action card and 1 objective card
    }

    fn move_units(&mut self, player_index: usize, units: &[u32], destination: Position) {
        for unit_id in units {
            self.move_unit(player_index, *unit_id, destination);
        }
    }

    fn move_unit(&mut self, player_index: usize, unit_id: u32, destination: Position) {
        let unit = self.players[player_index]
            .get_unit_mut(unit_id)
            .expect("the player should have all units to move");
        unit.position = destination;
        let terrain = self
            .map
            .tiles
            .get(&destination)
            .expect("the destination position should exist on the map")
            .clone();
        match terrain {
            Mountain => unit.restrict_movement(),
            Forest => unit.restrict_attack(),
            _ => (),
        };
    }

    fn undo_move_units(
        &mut self,
        player_index: usize,
        units: Vec<u32>,
        starting_position: Position,
    ) {
        let Some(unit) = units.first() else {
            return;
        };
        let destination = self.players[player_index]
            .get_unit(*unit)
            .expect("there should be at least one moved unit")
            .position;
        let terrain = self
            .map
            .tiles
            .get(&destination)
            .expect("the destination position should exist on the map")
            .clone();
        for unit_id in units {
            let unit = self.players[player_index]
                .get_unit_mut(unit_id)
                .expect("the player should have all units to move");
            unit.position = starting_position;
            match terrain {
                Mountain => unit.undo_movement_restriction(),
                Forest => unit.undo_attack_restriction(),
                _ => (),
            };
        }
    }

    ///
    /// # Panics
    ///
    /// Panics if the player does not have the unit
    pub fn kill_unit(&mut self, unit_id: u32, player_index: usize, killer: usize) {
        if let Some(unit) = self.players[player_index].remove_unit(unit_id) {
            if matches!(unit.unit_type, Leader) {
                let leader = self.players[player_index]
                    .active_leader
                    .take()
                    .expect("A player should have an active leader when having a leader unit");
                (leader.player_deinitializer)(self, player_index);
                self.players[killer].captured_leaders.push(leader.name);
            }
        }
    }

    pub fn set_player_index(&mut self, current_player_index: usize) {
        self.current_player_index = current_player_index;
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    action_log: Vec<ActionLogItem>,
    action_log_index: usize,
    log: Vec<String>,
    undo_limit: usize,
    played_once_per_turn_actions: Vec<CustomActionType>,
    actions_left: u32,
    successful_cultural_influence: bool,
    round: u32,
    age: u32,
    messages: Vec<String>,
    dice_roll_outcomes: Vec<u8>,
    dice_roll_log: Vec<u8>,
    dropped_players: Vec<usize>,
    wonders_left: Vec<String>,
    wonder_amount_left: usize,
    undo_context_stack: Vec<UndoContext>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CulturalInfluenceResolution {
    pub roll_boost_cost: u32,
    pub target_player_index: usize,
    pub target_city_position: Position,
    pub city_piece: Building,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Playing,
    StatusPhase(StatusPhaseState),
    Movement {
        movement_actions_left: u32,
        moved_units: Vec<u32>,
    },
    CulturalInfluenceResolution(CulturalInfluenceResolution),
    Combat(Combat),
    PlaceSettler {
        player_index: usize,
        movement_actions_left: u32,
        moved_units: Vec<u32>,
    },
    Finished,
}

impl GameState {
    #[must_use]
    pub fn settler_placer(&self) -> Option<usize> {
        match self {
            PlaceSettler {
                player_index,
                movement_actions_left: _,
                moved_units: _,
            } => Some(*player_index),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum UndoContext {
    FoundCity {
        settler: Unit,
    },
    Recruit {
        replaced_units: Vec<Unit>,
        replaced_leader: Option<String>,
    },
    Movement {
        starting_position: Option<Position>,
        movement_actions_left: u32,
        moved_units: Vec<u32>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<String>,
    data: GameData,
}

impl Messages {
    #[must_use]
    pub fn new(messages: Vec<String>, data: GameData) -> Self {
        Self { messages, data }
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
        utils,
        wonder::Wonder,
    };

    use super::{Game, GameState::Playing};

    #[must_use]
    pub fn test_game() -> Game {
        Game {
            state: Playing,
            players: Vec::new(),
            map: Map::new(HashMap::new()),
            starting_player_index: 0,
            current_player_index: 0,
            action_log: Vec::new(),
            action_log_index: 0,
            log: vec![
                String::from("The game has started"),
                String::from("Age 1 has started"),
                String::from("Round 1/3"),
            ],
            undo_limit: 0,
            played_once_per_turn_actions: Vec::new(),
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            dice_roll_outcomes: vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
            dice_roll_log: Vec::new(),
            dropped_players: Vec::new(),
            wonders_left: Vec::new(),
            wonder_amount_left: 0,
            undo_context_stack: Vec::new(),
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
        game.players[old].cities.push(City::new(old, position));
        game.build_wonder(wonder, position, old);
        game.players[old].construct(&Academy, position, None);
        game.players[old].construct(&Obelisk, position, None);

        assert!(utils::tests::eq_f32(
            8.0,
            game.players[old].victory_points()
        ));

        game.conquer_city(position, new, old);

        let c = game.players[new]
            .get_city_mut(position)
            .expect("player new should the city");
        assert_eq!(1, c.player_index);
        assert_eq!(Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        assert!(utils::tests::eq_f32(4.0, old.victory_points()));
        assert!(utils::tests::eq_f32(5.0, new.victory_points()));
        assert_eq!(0, old.wonders.len());
        assert_eq!(1, new.wonders.len());
        assert_eq!(1, old.influenced_buildings);
        assert_eq!(0, new.influenced_buildings);
    }
}
