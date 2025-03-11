use crate::ability_initializer::AbilityListeners;
use crate::combat::{CombatDieRoll, COMBAT_DIE_SIDES};
use crate::consts::{ACTIONS, NON_HUMAN_PLAYERS};
use crate::content::builtin;
use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::content::custom_phase_actions::{
    CurrentEventHandler, CurrentEventPlayer, CurrentEventState, CurrentEventType,
};
use crate::events::{Event, EventOrigin};
use crate::incident::PermanentIncidentEffect;
use crate::movement::{CurrentMove, MoveState};
use crate::pirates::get_pirates_player;
use crate::player_events::{CurrentEvent, CurrentEventInfo, PlayerEvents};
use crate::resource::check_for_waste;
use crate::status_phase::enter_status_phase;
use crate::unit::UnitType;
use crate::utils::Rng;
use crate::utils::Shuffle;
use crate::{
    action::Action,
    city::City,
    content::{civilizations, custom_actions::CustomActionType, wonders},
    map::{Map, MapData},
    player::{Player, PlayerData},
    position::Position,
    wonder::Wonder,
};
use itertools::Itertools;
use json_patch::PatchOperation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::vec;

pub struct Game {
    pub state: GameState,
    pub current_events: Vec<CurrentEventState>,
    // in turn order starting from starting_player_index and wrapping around
    pub players: Vec<Player>,
    pub map: Map,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub action_log: Vec<ActionLogItem>,
    pub action_log_index: usize,
    pub log: Vec<Vec<String>>,
    pub undo_limit: usize,
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    pub round: u32, // starts at 1
    pub age: u32,   // starts at 1
    pub messages: Vec<String>,
    pub rng: Rng,
    pub dice_roll_outcomes: Vec<u8>, // for testing
    pub dice_roll_log: Vec<u8>,
    pub dropped_players: Vec<usize>,
    pub wonders_left: Vec<Wonder>,
    pub wonder_amount_left: usize, // todo is this redundant?
    pub incidents_left: Vec<u8>,
    pub permanent_incident_effects: Vec<PermanentIncidentEffect>,
}

impl Clone for Game {
    fn clone(&self) -> Self {
        Self::from_data(self.cloned_data())
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.cloned_data() == other.cloned_data()
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
        let seed: &[u8] = seed.as_bytes();
        let mut buffer = [0u8; 16];
        buffer[..].copy_from_slice(&seed[..16]);
        let seed1 = u128::from_be_bytes(buffer);
        let mut buffer = [0u8; 16];
        buffer[..].copy_from_slice(&seed[16..]);
        let seed2 = u128::from_be_bytes(buffer);
        let seed = seed1 ^ seed2;
        let mut rng = Rng::from_seed(seed);

        let mut players = Vec::new();
        let mut civilizations = civilizations::get_all();
        for i in 0..player_amount {
            let civilization = rng.range(NON_HUMAN_PLAYERS, civilizations.len());
            players.push(Player::new(civilizations.remove(civilization), i));
        }

        let starting_player = rng.range(0, players.len());

        players.push(Player::new(
            civilizations::get_civilization(BARBARIANS).expect("civ not found"),
            players.len(),
        ));
        players.push(Player::new(
            civilizations::get_civilization(PIRATES).expect("civ not found"),
            players.len(),
        ));

        let mut wonders = wonders::get_all();
        wonders.shuffle(&mut rng);
        let wonder_amount = wonders.len();

        let map = if setup {
            Map::random_map(&mut players, &mut rng)
        } else {
            Map::new(HashMap::new())
        };

        let mut game = Self {
            state: GameState::Playing,
            current_events: Vec::new(),
            players,
            map,
            starting_player_index: starting_player,
            current_player_index: starting_player,
            action_log: Vec::new(),
            action_log_index: 0,
            log: [
                String::from("The game has started"),
                String::from("Age 1 has started"),
                String::from("Round 1/3"),
            ]
            .iter()
            .map(|s| vec![s.clone()])
            .collect(),
            undo_limit: 0,
            actions_left: ACTIONS,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("The game has started")],
            rng,
            dice_roll_outcomes: Vec::new(),
            dice_roll_log: Vec::new(),
            dropped_players: Vec::new(),
            wonders_left: wonders,
            wonder_amount_left: wonder_amount,
            incidents_left: Vec::new(),
            permanent_incident_effects: Vec::new(),
        };
        for i in 0..game.players.len() {
            builtin::init_player(&mut game, i);
        }

        game
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
            round: data.round,
            age: data.age,
            messages: data.messages,
            rng: Rng::from_seed_string(&data.rng),
            dice_roll_outcomes: data.dice_roll_outcomes,
            dice_roll_log: data.dice_roll_log,
            dropped_players: data.dropped_players,
            wonders_left: data
                .wonders_left
                .into_iter()
                .map(|wonder| wonders::get_wonder(&wonder))
                .collect(),
            wonder_amount_left: data.wonder_amount_left,
            incidents_left: data.incidents_left,
            permanent_incident_effects: data.permanent_incident_effects,
            current_events: data.current_events,
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
            current_events: self.current_events,
            players: self.players.into_iter().map(Player::data).collect(),
            map: self.map.data(),
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            action_log: self.action_log,
            action_log_index: self.action_log_index,
            log: self.log,
            undo_limit: self.undo_limit,
            actions_left: self.actions_left,
            successful_cultural_influence: self.successful_cultural_influence,
            round: self.round,
            age: self.age,
            messages: self.messages,
            rng: self.rng.seed.to_string(),
            dice_roll_outcomes: self.dice_roll_outcomes,
            dice_roll_log: self.dice_roll_log,
            dropped_players: self.dropped_players,
            wonders_left: self
                .wonders_left
                .into_iter()
                .map(|wonder| wonder.name)
                .collect(),
            wonder_amount_left: self.wonder_amount_left,
            incidents_left: self.incidents_left,
            permanent_incident_effects: self.permanent_incident_effects,
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> GameData {
        GameData {
            state: self.state.clone(),
            current_events: self.current_events.clone(),
            players: self.players.iter().map(Player::cloned_data).collect(),
            map: self.map.cloned_data(),
            starting_player_index: self.starting_player_index,
            current_player_index: self.current_player_index,
            action_log: self.action_log.clone(),
            action_log_index: self.action_log_index,
            log: self.log.clone(),
            undo_limit: self.undo_limit,
            actions_left: self.actions_left,
            successful_cultural_influence: self.successful_cultural_influence,
            round: self.round,
            age: self.age,
            messages: self.messages.clone(),
            rng: self.rng.seed.to_string(),
            dice_roll_outcomes: self.dice_roll_outcomes.clone(),
            dice_roll_log: self.dice_roll_log.clone(),
            dropped_players: self.dropped_players.clone(),
            wonders_left: self
                .wonders_left
                .iter()
                .map(|wonder| wonder.name.clone())
                .collect(),
            wonder_amount_left: self.wonder_amount_left,
            incidents_left: self.incidents_left.clone(),
            permanent_incident_effects: self.permanent_incident_effects.clone(),
        }
    }

    #[must_use]
    pub fn get_player(&self, player_index: usize) -> &Player {
        &self.players[player_index]
    }

    #[must_use]
    pub fn player_name(&self, player_index: usize) -> String {
        self.get_player(player_index).get_name()
    }

    #[must_use]
    pub fn get_player_mut(&mut self, player_index: usize) -> &mut Player {
        &mut self.players[player_index]
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
        self.get_player(player_index).get_city(position)
    }

    ///
    /// # Panics
    /// Panics if the city does not exist
    #[must_use]
    pub fn get_any_city(&self, position: Position) -> &City {
        self.try_get_any_city(position).expect("city not found")
    }

    #[must_use]
    pub fn try_get_any_city(&self, position: Position) -> Option<&City> {
        self.players
            .iter()
            .find_map(|player| player.try_get_city(position))
    }

    pub(crate) fn lock_undo(&mut self) {
        self.undo_limit = self.action_log_index;
        for a in &mut self.action_log {
            a.undo.clear();
        }
    }

    ///
    /// # Panics
    /// Panics if the player does not have events
    #[must_use]
    pub fn current_event(&self) -> &CurrentEventState {
        self.current_events.last().expect("state should exist")
    }

    #[must_use]
    pub(crate) fn current_event_mut(&mut self) -> &mut CurrentEventState {
        self.current_events.last_mut().expect("state should exist")
    }

    #[must_use]
    pub(crate) fn current_event_player(&self) -> &CurrentEventPlayer {
        &self.current_event().player
    }

    #[must_use]
    pub fn current_event_handler(&self) -> Option<&CurrentEventHandler> {
        self.current_events
            .last()
            .and_then(|s| s.player.handler.as_ref())
    }

    pub fn current_event_handler_mut(&mut self) -> Option<&mut CurrentEventHandler> {
        self.current_events
            .last_mut()
            .and_then(|s| s.player.handler.as_mut())
    }

    pub(crate) fn trigger_current_event_with_listener<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PlayerEvents) -> &mut CurrentEvent<V>,
        listeners: &AbilityListeners,
        event_type: &V,
        store_type: impl Fn(V) -> CurrentEventType,
        log: Option<&str>,
    ) -> bool
    where
        V: Clone,
    {
        for p in players {
            (listeners.initializer)(self, *p);
        }

        let return_early = self
            .trigger_current_event(players, event, event_type, store_type, log)
            .is_none();

        for p in players {
            (listeners.deinitializer)(self, *p);
        }
        return_early
    }

    pub(crate) fn trigger_current_event<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PlayerEvents) -> &mut CurrentEvent<V>,
        details: &V,
        to_event_type: impl Fn(V) -> CurrentEventType,
        log: Option<&str>,
    ) -> Option<CurrentEventType>
    where
        V: Clone,
    {
        let current_event_type = to_event_type(details.clone());
        if self
            .current_events
            .last()
            .is_none_or(|s| s.event_type != current_event_type)
        {
            if let Some(log) = log {
                self.add_info_log_group(log.to_string());
            }
            self.current_events
                .push(CurrentEventState::new(players[0], current_event_type));
        }
        let state = self.current_event();

        for player_index in Self::remaining_current_event_players(players, state) {
            let info = CurrentEventInfo {
                player: player_index,
            };
            self.trigger_event_with_game_value(player_index, event, &info, details);
            if self.current_event().player.handler.is_some() {
                return None;
            }
            let state = self.current_event_mut();
            state.players_used.push(player_index);
            if let Some(&p) = Self::remaining_current_event_players(players, state).first() {
                state.player = CurrentEventPlayer::new(p);
            }
        }
        self.current_events.pop().map(|s| s.event_type)
    }

    fn remaining_current_event_players(players: &[usize], state: &CurrentEventState) -> Vec<usize> {
        players
            .iter()
            .filter(|p| !state.players_used.contains(p))
            .copied()
            .collect_vec()
    }

    pub(crate) fn trigger_event_with_game_value<U, V>(
        &mut self,
        player_index: usize,
        event: fn(&mut PlayerEvents) -> &mut Event<Game, U, V>,
        info: &U,
        details: &V,
    ) {
        let e = event(&mut self.players[player_index].events).take();
        let _ = e.trigger(self, info, details);
        event(&mut self.players[player_index].events).set(e);
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.undo_limit < self.action_log_index
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.action_log_index < self.action_log.len()
    }
    pub(crate) fn is_pirate_zone(&self, position: Position) -> bool {
        if self.map.is_sea(position) {
            let pirate = get_pirates_player(self);
            if !pirate.get_units(position).is_empty() {
                return true;
            }
            return position
                .neighbors()
                .iter()
                .any(|n| !pirate.get_units(*n).is_empty());
        }
        false
    }

    #[must_use]
    pub fn enemy_player(&self, player_index: usize, position: Position) -> Option<usize> {
        self.players.iter().position(|player| {
            player.index != player_index
                && (!player.get_units(position).is_empty()
                    || player.try_get_city(position).is_some())
        })
    }

    pub fn add_info_log_group(&mut self, info: String) {
        self.log.push(vec![info]);
    }

    pub fn add_info_log_item(&mut self, info: &str) {
        let last_item_index = self.log.len() - 1;
        self.log[last_item_index].push(info.to_string());
    }

    pub fn add_to_last_log_item(&mut self, edit: &str) {
        let last_item_index = self.log.len() - 1;
        let vec = &mut self.log[last_item_index];
        let l = vec.len() - 1;
        vec[l] += edit;
    }

    ///
    /// # Panics
    /// Panics if the player does not have events
    pub fn next_player(&mut self) {
        check_for_waste(self);
        self.increment_player_index();
        self.add_info_log_group(format!(
            "It's {}'s turn",
            self.player_name(self.current_player_index)
        ));
        self.actions_left = ACTIONS;
        let lost_action = self.permanent_incident_effects.iter().position(
            |e| matches!(e, PermanentIncidentEffect::LooseAction(p) if *p == self.current_player_index),
        ).map(|i| self.permanent_incident_effects.remove(i));
        if lost_action.is_some() {
            self.add_info_log_item("Remove 1 action for Revolution");
            self.actions_left -= 1;
        }
        self.successful_cultural_influence = false;

        self.start_turn();
    }

    pub(crate) fn start_turn(&mut self) {
        self.trigger_current_event(
            &[self.current_player_index],
            |e| &mut e.on_turn_start,
            &(),
            |()| CurrentEventType::TurnStart,
            None,
        );
    }

    pub fn skip_dropped_players(&mut self) {
        if self.human_players_count() == 0 {
            return;
        }
        while self.dropped_players.contains(&self.current_player_index)
            && self.current_player_index != self.starting_player_index
        {
            self.increment_player_index();
        }
    }

    pub fn increment_player_index(&mut self) {
        // Barbarians and Pirates have the highest player indices
        self.current_player_index += 1;
        self.current_player_index %= self.human_players_count();
    }

    #[must_use]
    pub fn active_player(&self) -> usize {
        if let Some(e) = &self.current_events.last() {
            return e.player.index;
        }
        self.current_player_index
    }

    #[must_use]
    pub(crate) fn human_players_count(&self) -> usize {
        self.players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| self.is_active_human(i, p))
            .count()
    }

    ///
    /// # Panics
    /// Panics if the player is not human
    #[must_use]
    pub fn human_players(&self, first: usize) -> Vec<usize> {
        let mut all = self
            .players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| self.is_active_human(i, p))
            .collect_vec();
        let i = all
            .iter()
            .position(|&p| p == first)
            .expect("player should exist");
        all.rotate_left(i);
        all
    }

    fn is_active_human(&self, i: usize, p: &Player) -> Option<usize> {
        if p.civilization.is_human() && !self.dropped_players.contains(&i) {
            Some(i)
        } else {
            None
        }
    }

    pub fn next_turn(&mut self) {
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
            enter_status_phase(self);
            return;
        }
        self.add_info_log_group(format!("Round {}/3", self.round));
    }

    pub fn next_age(&mut self) {
        self.age += 1;
        self.current_player_index = self.starting_player_index;
        self.add_info_log_group(format!("Age {} has started", self.age));
        self.add_info_log_group(String::from("Round 1/3"));
    }

    pub(crate) fn end_game(&mut self) {
        let winner_player_index = self
            .players
            .iter()
            .enumerate()
            .max_by(|(_, player), (_, other)| player.compare_score(other, self))
            .expect("there should be at least one player in the game")
            .0;
        let winner_name = self.player_name(winner_player_index);
        self.add_info_log_group(format!("The game has ended. {winner_name} has won"));
        self.add_message("The game has ended");
    }

    pub(crate) fn get_next_dice_roll(&mut self) -> CombatDieRoll {
        self.lock_undo(); // dice rolls are not undoable
        let dice_roll = if self.dice_roll_outcomes.is_empty() {
            self.rng.range(0, 12) as u8
        } else {
            // only for testing
            self.dice_roll_outcomes
                .pop()
                .expect("dice roll outcomes should not be empty")
        };
        self.dice_roll_log.push(dice_roll);
        COMBAT_DIE_SIDES[dice_roll as usize].clone()
    }

    fn add_message(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }

    pub fn drop_player(&mut self, player_index: usize) {
        self.dropped_players.push(player_index);
        self.add_message(&format!(
            "{} has left the game",
            self.player_name(player_index)
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
    pub fn get_available_custom_actions(
        &self,
        player_index: usize,
    ) -> Vec<(CustomActionType, EventOrigin)> {
        self.players[self.current_player_index]
            .custom_actions
            .clone()
            .into_iter()
            .filter(|(action, _)| {
                !self
                    .get_player(player_index)
                    .played_once_per_turn_actions
                    .contains(action)
                    && action.is_available(self, player_index)
            })
            .collect()
    }

    #[must_use]
    pub fn is_custom_action_available(
        &self,
        player_index: usize,
        action: &CustomActionType,
    ) -> bool {
        self.get_available_custom_actions(player_index)
            .iter()
            .any(|(a, _)| a == action)
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
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn build_wonder(&mut self, wonder: Wonder, city_position: Position, player_index: usize) {
        self.players[player_index].trigger_player_event(
            |events| &mut events.on_construct_wonder,
            &city_position,
            &wonder,
        );
        let wonder = wonder;
        (wonder.listeners.initializer)(self, player_index);
        (wonder.listeners.one_time_initializer)(self, player_index);
        let player = &mut self.players[player_index];
        player.wonders_build.push(wonder.name.clone());
        player
            .get_city_mut(city_position)
            .pieces
            .wonders
            .push(wonder);
    }

    ///
    /// # Panics
    ///
    /// Panics if the player does not have the unit
    pub fn kill_unit(&mut self, unit_id: u32, player_index: usize, killer: Option<usize>) {
        let unit = self.players[player_index].remove_unit(unit_id);
        if matches!(unit.unit_type, UnitType::Leader) {
            let leader = self.players[player_index]
                .active_leader
                .take()
                .expect("A player should have an active leader when having a leader unit");
            Player::with_leader(&leader, self, player_index, |game, leader| {
                (leader.listeners.deinitializer)(game, player_index);
            });
            if let Some(killer) = killer {
                self.players[killer].captured_leaders.push(leader);
            }
        }
        if let GameState::Movement(m) = &mut self.state {
            if let CurrentMove::Fleet { units } = &mut m.current_move {
                units.retain(|&id| id != unit_id);
            }
        }
    }

    pub fn set_player_index(&mut self, current_player_index: usize) {
        self.current_player_index = current_player_index;
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    state: GameState,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    current_events: Vec<CurrentEventState>,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    action_log: Vec<ActionLogItem>,
    action_log_index: usize,
    log: Vec<Vec<String>>,
    undo_limit: usize,
    actions_left: u32,
    successful_cultural_influence: bool,
    round: u32,
    age: u32,
    messages: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dice_roll_outcomes: Vec<u8>, // for testing purposes
    #[serde(default)]
    #[serde(skip_serializing_if = "is_string_zero")]
    rng: String,
    dice_roll_log: Vec<u8>,
    dropped_players: Vec<usize>,
    wonders_left: Vec<String>,
    wonder_amount_left: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    incidents_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    permanent_incident_effects: Vec<PermanentIncidentEffect>,
}

fn is_string_zero(s: &String) -> bool {
    s == "0"
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Playing,
    Movement(MoveState),
    Finished,
}

impl GameState {
    #[must_use]
    pub fn is_playing(&self) -> bool {
        matches!(self, GameState::Playing)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogItem {
    pub action: Action,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undo: Vec<PatchOperation>,
}

impl ActionLogItem {
    #[must_use]
    pub fn new(action: Action) -> Self {
        Self {
            action,
            undo: Vec::new(),
        }
    }
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
