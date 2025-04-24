use crate::ability_initializer::AbilityListeners;
use crate::action::update_stats;
use crate::cache::Cache;
use crate::combat_roll::{COMBAT_DIE_SIDES, CombatDieRoll};
use crate::consts::ACTIONS;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{
    PersistentEventHandler, PersistentEventPlayer, PersistentEventState, PersistentEventType,
};
use crate::events::{Event, EventOrigin};
use crate::log::{
    ActionLogAge, ActionLogPlayer, ActionLogRound, current_player_turn_log,
    current_player_turn_log_mut,
};
use crate::movement::MoveState;
use crate::objective_card::present_instant_objective_cards;
use crate::pirates::get_pirates_player;
use crate::player::CostTrigger;
use crate::player_events::{
    PersistentEvent, PersistentEventInfo, PersistentEvents, PlayerEvents, TransientEvents,
};
use crate::resource::check_for_waste;
use crate::status_phase::enter_status_phase;
use crate::utils;
use crate::utils::Rng;
use crate::{
    city::City,
    content::custom_actions::CustomActionType,
    map::{Map, MapData},
    player::{Player, PlayerData},
    position::Position,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::vec;

pub struct Game {
    pub cache: Cache,
    pub state: GameState,
    pub events: Vec<PersistentEventState>,
    // in turn order starting from starting_player_index and wrapping around
    pub players: Vec<Player>,
    pub map: Map,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub action_log: Vec<ActionLogAge>,
    // index for the next action log
    pub action_log_index: usize,
    pub log: Vec<Vec<String>>,
    pub undo_limit: usize,
    pub ai_mode: bool,
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    pub round: u32, // starts at 1
    pub age: u32,   // starts at 1
    pub messages: Vec<String>,
    pub rng: Rng,
    pub dice_roll_outcomes: Vec<u8>, // for testing
    pub dice_roll_log: Vec<u8>,
    pub dropped_players: Vec<usize>,
    pub wonders_left: Vec<String>,
    pub action_cards_left: Vec<u8>,
    pub objective_cards_left: Vec<u8>,
    pub incidents_left: Vec<u8>,
    pub permanent_effects: Vec<PermanentEffect>,
}

impl Clone for Game {
    fn clone(&self) -> Self {
        let mut game = Self::from_data(self.cloned_data(), self.cache.clone());
        game.ai_mode = self.ai_mode;
        game
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.cloned_data() == other.cloned_data()
    }
}

impl Game {
    ///
    ///
    /// # Panics
    ///
    /// Panics if any wonder does not exist
    #[must_use]
    pub fn from_data(data: GameData, cache: Cache) -> Self {
        let mut game = Self {
            cache,
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
            ai_mode: false,
            round: data.round,
            age: data.age,
            messages: data.messages,
            rng: Rng::from_seed_string(&data.rng),
            dice_roll_outcomes: data.dice_roll_outcomes,
            dice_roll_log: data.dice_roll_log,
            dropped_players: data.dropped_players,
            wonders_left: data.wonders_left,
            action_cards_left: data.action_cards_left,
            objective_cards_left: data.objective_cards_left,
            incidents_left: data.incidents_left,
            permanent_effects: data.permanent_effects,
            events: data.events,
        };
        for player in data.players {
            Player::initialize_player(player, &mut game);
        }
        update_stats(&mut game);
        game
    }

    #[must_use]
    pub fn data(self) -> GameData {
        GameData {
            state: self.state,
            events: self.events,
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
            wonders_left: self.wonders_left,
            action_cards_left: self.action_cards_left,
            objective_cards_left: self.objective_cards_left,
            incidents_left: self.incidents_left,
            permanent_effects: self.permanent_effects,
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> GameData {
        GameData {
            state: self.state.clone(),
            events: self.events.clone(),
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
            wonders_left: self.wonders_left.clone(),
            action_cards_left: self.action_cards_left.clone(),
            objective_cards_left: self.objective_cards_left.clone(),
            incidents_left: self.incidents_left.clone(),
            permanent_effects: self.permanent_effects.clone(),
        }
    }

    #[must_use]
    pub fn player(&self, player_index: usize) -> &Player {
        &self.players[player_index]
    }

    #[must_use]
    pub fn player_name(&self, player_index: usize) -> String {
        self.player(player_index).get_name()
    }

    #[must_use]
    pub fn player_mut(&mut self, player_index: usize) -> &mut Player {
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
    pub fn city(&self, player_index: usize, position: Position) -> &City {
        self.player(player_index).get_city(position)
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
        if !self.ai_mode {
            self.undo_limit = self.action_log_index;
            current_player_turn_log_mut(self).clear_undo();
        }
    }

    ///
    /// # Panics
    /// Panics if the player does not have events
    #[must_use]
    pub fn current_event(&self) -> &PersistentEventState {
        self.events.last().expect("state should exist")
    }

    #[must_use]
    pub(crate) fn current_event_mut(&mut self) -> &mut PersistentEventState {
        self.events.last_mut().expect("state should exist")
    }

    #[must_use]
    pub fn current_event_handler(&self) -> Option<&PersistentEventHandler> {
        self.events.last().and_then(|s| s.player.handler.as_ref())
    }

    pub fn current_event_handler_mut(&mut self) -> Option<&mut PersistentEventHandler> {
        self.events
            .last_mut()
            .and_then(|s| s.player.handler.as_mut())
    }

    pub(crate) fn trigger_persistent_event_with_listener<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PersistentEvents) -> &mut PersistentEvent<V>,
        listeners: &AbilityListeners,
        event_type: V,
        store_type: impl Fn(V) -> PersistentEventType,
        log: Option<&str>,
        next_player: fn(&mut V) -> (),
    ) -> Option<V>
    where
        V: Clone + PartialEq,
    {
        for p in players {
            listeners.init(self, *p);
        }

        let result = self.trigger_persistent_event_ext(
            players,
            event,
            event_type,
            store_type,
            log,
            next_player,
        );

        for p in players {
            listeners.deinit(self, *p);
        }
        result
    }

    #[must_use]
    pub(crate) fn trigger_persistent_event<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PersistentEvents) -> &mut PersistentEvent<V>,
        value: V,
        to_event_type: impl Fn(V) -> PersistentEventType,
    ) -> Option<V>
    where
        V: Clone + PartialEq,
    {
        self.trigger_persistent_event_ext(players, event, value, to_event_type, None, |_| {})
    }

    #[must_use]
    pub(crate) fn trigger_persistent_event_ext<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PersistentEvents) -> &mut PersistentEvent<V>,
        mut value: V,
        to_event_type: impl Fn(V) -> PersistentEventType,
        log: Option<&str>,
        next_player: fn(&mut V) -> (),
    ) -> Option<V>
    where
        V: Clone + PartialEq,
    {
        let current_event_type = to_event_type(value.clone());
        if self
            .events
            .last()
            .is_none_or(|s| s.event_type != current_event_type)
        {
            if let Some(log) = log {
                self.add_info_log_group(log.to_string());
            }
            self.events
                .push(PersistentEventState::new(players[0], current_event_type));
        }

        let event_index = self.events.len() - 1;

        for player_index in Self::remaining_persistent_event_players(players, self.current_event())
        {
            let info = PersistentEventInfo {
                player: player_index,
            };
            self.trigger_event_with_game_value(
                player_index,
                move |e| event(&mut e.persistent),
                &info,
                &(),
                &mut value,
            );

            if self.current_event().player.handler.is_some() {
                self.events[event_index].event_type = to_event_type(value);
                return None;
            }
            let state = self.current_event_mut();
            state.players_used.push(player_index);
            if let Some(&p) = Self::remaining_persistent_event_players(players, state).first() {
                state.player = PersistentEventPlayer::new(p);
                next_player(&mut value);
            }
        }
        self.events.pop();

        if self.events.is_empty() {
            present_instant_objective_cards(self);
        }

        Some(value)
    }

    fn remaining_persistent_event_players(
        players: &[usize],
        state: &PersistentEventState,
    ) -> Vec<usize> {
        players
            .iter()
            .filter(|p| !state.players_used.contains(p))
            .copied()
            .collect_vec()
    }

    pub(crate) fn trigger_transient_event_with_game_value<U, V>(
        &mut self,
        player_index: usize,
        event: fn(&mut TransientEvents) -> &mut Event<Game, U, V>,
        info: &U,
        details: &V,
    ) {
        self.trigger_event_with_game_value(
            player_index,
            move |e| event(&mut e.transient),
            info,
            details,
            &mut (),
        );
    }

    fn trigger_event_with_game_value<U, V, W>(
        &mut self,
        player_index: usize,
        event: impl Fn(&mut PlayerEvents) -> &mut Event<Game, U, V, W>,
        info: &U,
        details: &V,
        extra_value: &mut W,
    ) where
        W: Clone + PartialEq,
    {
        let e = event(&mut self.players[player_index].events).take();
        e.trigger(self, info, details, extra_value);
        event(&mut self.players[player_index].events).set(e);
    }

    #[must_use]
    pub(crate) fn execute_cost_trigger(&self) -> CostTrigger {
        if self.ai_mode {
            CostTrigger::NoModifiers
        } else {
            CostTrigger::WithModifiers
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.ai_mode && self.undo_limit < self.action_log_index
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.ai_mode && self.action_log_index < current_player_turn_log(self).items.len()
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

    pub(crate) fn start_turn(&mut self) {
        self.action_log
            .last_mut()
            .expect("action log should exist")
            .rounds
            .last_mut()
            .expect("round should exist")
            .players
            .push(ActionLogPlayer::new(self.current_player_index));
        self.action_log_index = 0;
        self.undo_limit = 0;

        self.add_info_log_group(format!(
            "It's {}'s turn",
            self.player_name(self.current_player_index)
        ));
        self.actions_left = ACTIONS;
        let lost_action = self
            .permanent_effects
            .iter()
            .position(
                |e| matches!(e, PermanentEffect::LoseAction(p) if *p == self.current_player_index),
            )
            .map(|i| self.permanent_effects.remove(i));
        if lost_action.is_some() {
            self.add_info_log_item("Remove 1 action for Revolution");
            self.actions_left -= 1;
        }
        self.successful_cultural_influence = false;

        self.on_start_turn();
    }

    pub(crate) fn on_start_turn(&mut self) {
        let _ = self.trigger_persistent_event(
            &[self.current_player_index],
            |e| &mut e.turn_start,
            (),
            |()| PersistentEventType::TurnStart,
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
        if let Some(e) = &self.events.last() {
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
        self.player_mut(self.current_player_index).end_turn();
        for i in &mut current_player_turn_log_mut(self).items {
            i.undo.clear();
        }
        check_for_waste(self);
        self.increment_player_index();
        self.skip_dropped_players();
        if self.current_player_index == self.starting_player_index {
            self.next_round();
        } else {
            self.start_turn();
        }
    }

    fn next_round(&mut self) {
        self.round += 1;
        self.skip_dropped_players();
        if self.round > 3 {
            enter_status_phase(self);
            return;
        }
        self.add_info_log_group(format!("Round {}/3", self.round));
        self.action_log
            .last_mut()
            .expect("action log should exist")
            .rounds
            .push(ActionLogRound::new());
        self.start_turn();
    }

    pub fn next_age(&mut self) {
        self.age += 1;
        self.round = 0;
        self.current_player_index = self.starting_player_index;
        self.add_info_log_group(format!("Age {} has started", self.age));
        self.action_log.push(ActionLogAge::new());
        self.next_round();
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
        self.state = GameState::Finished;
    }

    pub(crate) fn next_dice_roll(&mut self) -> CombatDieRoll {
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
    pub fn available_custom_actions(
        &self,
        player_index: usize,
    ) -> Vec<(CustomActionType, EventOrigin)> {
        self.player(self.current_player_index)
            .custom_actions
            .clone()
            .into_iter()
            .filter(|(t, _)| {
                if matches!(
                    t,
                    CustomActionType::ArtsInfluenceCultureAttempt
                        | CustomActionType::FreeEconomyCollect
                        | CustomActionType::VotingIncreaseHappiness
                ) {
                    // returned as part of "base_or_custom_available"
                    return false;
                }

                t.is_available(self, player_index)
            })
            .collect()
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
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    state: GameState,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<PersistentEventState>,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_log: Vec<ActionLogAge>,
    action_log_index: usize,
    log: Vec<Vec<String>>,
    undo_limit: usize,
    actions_left: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dice_roll_log: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dropped_players: Vec<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonders_left: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_cards_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    objective_cards_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    incidents_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    permanent_effects: Vec<PermanentEffect>,
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
