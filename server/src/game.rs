use crate::action::lose_action;
use crate::cache::Cache;
use crate::combat_roll::{COMBAT_DIE_SIDES, CombatDieRoll};
use crate::consts::ACTIONS;
use crate::content::custom_actions::{CustomActionExecution, CustomActionInfo};
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{
    PersistentEventHandler, PersistentEventState, PersistentEventType,
    TriggerPersistentEventParams, trigger_persistent_event_ext,
};
use crate::events::{Event, EventOrigin, EventPlayer};
use crate::game_data::GameData;
use crate::log::{
    ActionLogAge, add_player_log, add_round_log, current_player_turn_log,
    current_player_turn_log_mut,
};
use crate::movement::MoveState;
use crate::pirates::get_pirates_player;
use crate::player::{CostTrigger, end_turn};
use crate::player_events::{
    PersistentEvent, PersistentEvents, TransientEvents, trigger_event_with_game_value,
};
use crate::resource::check_for_waste;
use crate::status_phase::enter_status_phase;
use crate::utils::Rng;
use crate::victory_points::compare_score;
use crate::wonder::Wonder;
use crate::{city::City, game_data, map::Map, player::Player, position::Position};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::vec;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub enum UndoOption {
    // prevent undoing when secret information is revealed (default)
    #[default]
    ProtectSecrets,
    // allow undoing any action when the same player is playing
    SamePlayer,
}

impl UndoOption {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &UndoOption::ProtectSecrets
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct GameOptions {
    #[serde(default)]
    #[serde(skip_serializing_if = "UndoOption::is_default")]
    pub undo: UndoOption,
}

impl GameOptions {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &GameOptions::default()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GameContext {
    Play,
    AI,
    Replay,
}

pub struct Game {
    pub cache: Cache,
    pub context: GameContext, // trasient
    pub options: GameOptions,
    pub version: u16, // JSON schema version
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
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    pub round: u32, // starts at 1
    pub age: u32,   // starts at 1
    pub messages: Vec<String>,
    pub seed: String,
    pub rng: Rng,
    pub dice_roll_outcomes: Vec<u8>, // for testing
    pub dice_roll_log: Vec<u8>,
    pub dropped_players: Vec<usize>,
    pub wonders_left: Vec<Wonder>,
    pub action_cards_left: Vec<u8>,
    pub action_cards_discarded: Vec<u8>,
    pub objective_cards_left: Vec<u8>,
    pub incidents_left: Vec<u8>,
    pub incidents_discarded: Vec<u8>,
    pub permanent_effects: Vec<PermanentEffect>,
}

impl Clone for Game {
    fn clone(&self) -> Self {
        Self::from_data(self.cloned_data(), self.cache.clone(), self.context.clone())
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.cloned_data() == other.cloned_data()
    }
}

impl Game {
    #[must_use]
    pub fn from_data(data: GameData, cache: Cache, context: GameContext) -> Self {
        game_data::from_data(data, cache, context)
    }

    #[must_use]
    pub fn data(self) -> GameData {
        game_data::data(self)
    }

    #[must_use]
    pub fn cloned_data(&self) -> GameData {
        game_data::cloned_data(self)
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
    pub(crate) fn get_any_city_mut(&mut self, position: Position) -> &mut City {
        self.players
            .iter_mut()
            .find_map(|player| player.try_get_city_mut(position))
            .expect("city not found")
    }

    #[must_use]
    pub fn try_get_any_city(&self, position: Position) -> Option<&City> {
        self.players
            .iter()
            .find_map(|player| player.try_get_city(position))
    }

    pub(crate) fn information_revealed(&mut self) {
        if self.options.undo == UndoOption::ProtectSecrets {
            self.lock_undo();
        }
    }

    pub(crate) fn player_changed(&mut self) {
        self.lock_undo();
    }

    fn lock_undo(&mut self) {
        if self.context != GameContext::AI {
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
        trigger_persistent_event_ext(
            self,
            players,
            event,
            value,
            to_event_type,
            TriggerPersistentEventParams::default(),
        )
    }

    pub(crate) fn trigger_transient_event_with_game_value<U, V>(
        &mut self,
        player_index: usize,
        event: fn(&mut TransientEvents) -> &mut Event<Game, U, V>,
        info: &U,
        details: &V,
    ) {
        trigger_event_with_game_value(
            self,
            player_index,
            move |e| event(&mut e.transient),
            info,
            details,
            &mut (),
        );
    }

    #[must_use]
    pub(crate) fn execute_cost_trigger(&self) -> CostTrigger {
        if self.context == GameContext::AI {
            CostTrigger::NoModifiers
        } else {
            CostTrigger::WithModifiers
        }
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.context != GameContext::AI && self.undo_limit < self.action_log_index
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.context != GameContext::AI
            && self.action_log_index < current_player_turn_log(self).actions.len()
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

    pub fn log(&mut self, player: usize, origin: &EventOrigin, message: &str) {
        let prefix = format!("{}: {}: ", self.player_name(player), origin.name(self));
        let last_item_index = self.log.len() - 1;
        let current = &mut self.log[last_item_index];
        for c in current.iter_mut() {
            if c.starts_with(&prefix) {
                use std::fmt::Write as _;
                let _ = write!(c, ", {message}");
                return;
            }
        }
        current.push(format!("{prefix}{message}"));
    }

    pub(crate) fn start_turn(&mut self) {
        let player = self.current_player_index;
        add_player_log(self, player);
        self.action_log_index = 0;
        self.undo_limit = 0;

        self.add_info_log_group(format!("It's {}'s turn", self.player_name(player)));
        self.actions_left = ACTIONS;
        let lost_action = self
            .permanent_effects
            .iter()
            .position(|e| matches!(e, PermanentEffect::RevolutionLoseAction(p) if *p == player))
            .map(|i| self.permanent_effects.remove(i));
        if lost_action.is_some() {
            lose_action(
                self,
                &EventPlayer::from_player(
                    player,
                    self,
                    EventOrigin::Ability("Revolution".to_string()),
                ),
            );
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
        end_turn(self, self.current_player_index);
        for i in &mut current_player_turn_log_mut(self).actions {
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
        add_round_log(self, self.round);
        self.start_turn();
    }

    pub fn next_age(&mut self) {
        self.age += 1;
        self.round = 0;
        self.current_player_index = self.starting_player_index;
        let m = format!("Age {} has started", self.age);
        self.add_message(&m);
        self.add_info_log_group(m);
        self.action_log.push(ActionLogAge::new(self.age));
        self.next_round();
    }

    pub(crate) fn end_game(&mut self) {
        let winner_player_index = self
            .players
            .iter()
            .enumerate()
            .max_by(|(_, player), (_, other)| compare_score(player, other, self))
            .expect("there should be at least one player in the game")
            .0;
        let winner_name = self.player_name(winner_player_index);
        let m = format!("The game has ended. {winner_name} has won");
        self.add_message(&m);
        self.add_info_log_group(m);
        self.state = GameState::Finished;
    }

    pub(crate) fn next_dice_roll(&mut self) -> CombatDieRoll {
        self.information_revealed();
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
    pub fn available_custom_actions(&self, player_index: usize) -> Vec<CustomActionInfo> {
        self.player(player_index)
            .custom_actions
            .values()
            .filter(|&c| {
                if matches!(c.execution, CustomActionExecution::Modifier(_)) {
                    // returned as part of "base_or_custom_available"
                    return false;
                }

                c.action
                    .playing_action_type()
                    .is_available(self, player_index)
                    .is_ok()
            })
            .cloned()
            .collect_vec()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Playing,
    Movement(MoveState),
    Finished,
}
