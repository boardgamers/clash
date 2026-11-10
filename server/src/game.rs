use crate::action::lose_action;
use crate::cache::Cache;
use crate::combat_roll::{COMBAT_DIE_SIDES, CombatDieRoll};
use crate::consts::ACTIONS;
use crate::content::custom_actions::{SpecialActionExecution, SpecialActionInfo};
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::{
    PersistentEventHandler, PersistentEventState, PersistentEventType,
    TriggerPersistentEventParams, trigger_persistent_event_ext,
};
use crate::events::{Event, EventOrigin, EventPlayer};
use crate::game_data::GameData;
use crate::log::{
    ActionLogAge, ActionLogEntry, ActionLogItem, TurnType, add_round_log,
    add_start_turn_action_if_needed, add_turn_log, current_action_log_mut, current_turn_log,
    current_turn_log_mut,
};
use crate::movement::MoveState;
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
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub enum CivSetupOption {
    #[default]
    Random,
    ChooseCivilization,
    DraftThree,
}

impl CivSetupOption {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &CivSetupOption::Random
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub enum PatchOption {
    #[default]
    Standard,
    BalancePatch,
}

impl PatchOption {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &PatchOption::Standard
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub enum GameLength {
    #[default]
    Standard,
    Epic,
    Shorter,
    RollToEnd,
    WonderfulEnding,
}

impl GameLength {
    #[must_use]
    pub fn ages(&self) -> u32 {
        match self {
            Self::Standard => crate::consts::AGES,
            Self::Epic => 8,
            Self::Shorter => 4,
            Self::RollToEnd | Self::WonderfulEnding => 7,
        }
    }

    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &Self::Standard
    }
}

/// The printed seven-age track's final five rounds.
#[must_use]
pub fn variable_end_threshold(age: u32, round: u32) -> Option<u8> {
    match (age, round) {
        (6, 2) => Some(6),
        (6, 3) => Some(5),
        (7, 1) => Some(4),
        (7, 2) => Some(3),
        (7, 3) => Some(2),
        _ => None,
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum VariableEnd {
    RollToEnd,
    WonderfulEnding,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct GameOptions {
    // Read old saves only. Cleared on load; unrestricted undo is no longer a rule.
    #[serde(default, rename = "undo", skip_serializing)]
    pub legacy_undo: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "CivSetupOption::is_default")]
    pub civilization: CivSetupOption,
    #[serde(default)]
    #[serde(skip_serializing_if = "PatchOption::is_default")]
    pub patch: PatchOption,
    #[serde(default, skip_serializing_if = "GameLength::is_default")]
    pub length: GameLength,
    #[serde(default, skip_serializing_if = "GameVariant::is_default")]
    pub variant: GameVariant,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub enum GameVariant {
    #[default]
    Standard,
    Builder,
}

impl GameVariant {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &Self::Standard
    }
}

impl GameOptions {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &GameOptions::default()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GameContext {
    CombatPreview,
    Play,
    AI,
    Replay,
}

pub struct Game {
    pub cache: Cache,
    pub context: GameContext, // transient
    pub options: GameOptions,
    pub version: u16, // JSON schema version
    pub state: GameState,
    pub events: Vec<PersistentEventState>,
    pub early_choices: Vec<crate::early_choices::SavedChoice>,
    pub early_choice_update: bool,
    /// A private Calendar prompt, not yet part of the committed action history.
    pub uncommitted_calendar: bool,
    /// Live private disclosure; not restored by undo patches.
    pub calendar_knowledge: Option<crate::calendar_knowledge::CalendarKnowledge>,
    /// Filtered states must never create knowledge from shuffled hidden piles.
    pub secrets_stripped: bool,
    pub action_plan: Option<crate::action_queue::Plan>,
    // in turn order starting from starting_player_index and wrapping around
    pub players: Vec<Player>,
    pub map: Map,
    pub civilization_draft: Option<crate::game_setup::CivilizationDraft>,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub log: Vec<ActionLogAge>,
    pub board_history: crate::board_history::BoardHistory,
    /// Monotonic platform credits; deliberately outside the undoable position.
    pub(crate) time_increments: Vec<u64>,
    /// Transient signal for one saved atomic action, not a persisted undo cursor.
    pub(crate) timing_revealed: bool,
    /// One top incident Calendar could inspect before this atomic action; never saved.
    pub(crate) calendar_incident_undo: Option<u8>,
    // index for the next action log
    pub log_index: usize,
    pub undo_limit: usize,
    pub actions_left: u32,
    pub successful_cultural_influence: bool,
    /// Set by a variable ending; `WonderfulEnding` waits until this round completes.
    pub variable_end: Option<VariableEnd>,
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
    pub custom_ui_elements: HashMap<String, UIElement>,
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
        self.timing_revealed = true;
        self.lock_undo();
    }

    pub(crate) fn player_changed(&mut self) {
        self.lock_undo();
    }

    pub(crate) fn lock_undo(&mut self) {
        if self.context != GameContext::AI {
            self.undo_limit = self.log_index;
            current_turn_log_mut(self).clear_undo();
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

    pub(crate) fn trigger_transient_event_with_game_value<U: Clone, V: Clone>(
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
        self.context != GameContext::AI
            && (self.uncommitted_calendar || self.undo_limit < self.log_index)
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.context != GameContext::AI
            && !self.uncommitted_calendar
            && self.log_index < current_turn_log(self).actions.len()
    }

    pub(crate) fn is_pirate_zone(&self, position: Position) -> bool {
        if self.map.is_sea(position) {
            let Some(pirate) = self.players.iter().find(|p| p.civilization.is_pirates()) else {
                return false;
            };
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
                && !(player.civilization.is_pirates()
                    && self
                        .player(player_index)
                        .has_special_advance(crate::special_advance::SpecialAdvance::PirateAllies)
                    && !self
                        .player(player_index)
                        .event_info
                        .contains_key("Attack Pirates"))
                && (!player.get_units(position).is_empty()
                    || player.try_get_city(position).is_some())
        })
    }

    #[must_use]
    pub fn can_attack_player(&self, attacker: usize, defender: usize) -> bool {
        self.options.variant != GameVariant::Builder
            || !self.player(attacker).is_human()
            || !self.player(defender).is_human()
    }

    pub fn add_log_item(&mut self, item: ActionLogItem) {
        current_action_log_mut(self).items.push(item);
    }

    pub fn log(&mut self, player: usize, origin: &EventOrigin, message: &str) {
        self.add_log_item(ActionLogItem::new(
            player,
            ActionLogEntry::message(message.to_string()),
            origin.clone(),
            vec![],
        ));
    }

    pub(crate) fn start_turn(&mut self) {
        let player = self.current_player_index;
        add_turn_log(self, TurnType::Player(player));
        self.start_turn_after_log();
    }

    fn start_turn_after_log(&mut self) {
        let player = self.current_player_index;
        self.actions_left = ACTIONS;
        let lost_action = self
            .permanent_effects
            .iter()
            .position(|e| matches!(e, PermanentEffect::RevolutionLoseAction(p) if *p == player))
            .map(|i| self.permanent_effects.remove(i));
        if lost_action.is_some() {
            add_start_turn_action_if_needed(self, player);
            lose_action(
                self,
                &EventPlayer::new(player, EventOrigin::Ability("Revolution".to_string())),
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
        // Seat indices remain stable when a player drops.
        self.current_player_index %= self.players.iter().filter(|p| p.is_human()).count();
    }

    #[must_use]
    pub fn active_player(&self) -> usize {
        if let Some(e) = &self.events.last() {
            return crate::content::civilizations::celts::delegated_player(self, e.player.index);
        }
        self.current_player_index
    }

    /// All players who may submit now; setup choices are simultaneous.
    #[must_use]
    pub fn active_players(&self) -> Vec<usize> {
        if let Some(draft) = &self.civilization_draft {
            return draft
                .ready
                .iter()
                .enumerate()
                .filter_map(|(i, ready)| (!ready).then_some(i))
                .collect();
        }
        vec![self.active_player()]
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
    pub fn human_players_sorted(&self, first: usize) -> Vec<usize> {
        let mut all = self.human_player_ids();
        let i = all
            .iter()
            .position(|&p| p == first)
            .expect("player should exist");
        all.rotate_left(i);
        all
    }

    #[must_use]
    pub fn human_player_ids(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| self.is_active_human(i, p))
            .collect_vec()
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
        for i in &mut current_turn_log_mut(self).actions {
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
        if self.variable_end == Some(VariableEnd::WonderfulEnding) {
            enter_status_phase(self);
            return;
        }
        self.round += 1;
        self.skip_dropped_players();
        if self.round > 3 {
            enter_status_phase(self);
            return;
        }
        add_round_log(self, self.round);
        if self.options.length == GameLength::RollToEnd
            && let Some(threshold) = variable_end_threshold(self.age, self.round)
        {
            // Establish the log before rolling, but do not run any turn-start abilities.
            add_turn_log(self, TurnType::Player(self.current_player_index));
            add_start_turn_action_if_needed(self, self.current_player_index);
            let roll = self.next_dice_roll().value;
            self.log(
                self.current_player_index,
                &EventOrigin::Ability("Roll to End".into()),
                &format!("Roll to End: {roll} (end on {threshold}–6)"),
            );
            if roll >= threshold {
                self.variable_end = Some(VariableEnd::RollToEnd);
                enter_status_phase(self);
            } else {
                self.start_turn_after_log();
            }
            // Final-status / turn-start logging can replace the current turn.
            // Keep the reveal barrier in that new log too (including when the
            // first-player chooser is also the next active player).
            self.lock_undo();
            return;
        }
        self.start_turn();
    }

    pub fn next_age(&mut self) {
        self.age += 1;
        self.round = 0;
        self.current_player_index = self.starting_player_index;
        self.add_message(&format!("Age {} has started", self.age));
        self.log.push(ActionLogAge::new(self.age));
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
        add_start_turn_action_if_needed(self, 0);
        EventPlayer::new(
            winner_player_index,
            EventOrigin::Ability("having the most points".to_string()),
        )
        .log(self, "wins the game");
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

    /// End by withdrawal independently of points or pending choices.
    pub(crate) fn finish_after_drop(&mut self) -> bool {
        if self.dropped_players.is_empty() || self.human_players_count() > 1 {
            return false;
        }
        if matches!(self.state, GameState::Finished) {
            return true;
        }
        self.events.clear();
        if let Some(winner) = self.human_player_ids().first().copied() {
            self.current_player_index = winner;
            self.add_message(&format!(
                "{} wins: all other players have left the game",
                self.player_name(winner)
            ));
        }
        self.state = GameState::Finished;
        true
    }

    /// Cancel only an unsubmitted, standalone Banking reward. Other turn-start
    /// handlers include mandatory payments and shared effects; they are not safe
    /// to discard generically, nor are nested persistent-event continuations.
    fn dropped_banking_player(&self) -> Option<usize> {
        use crate::content::persistent_events::PersistentEventRequest;
        use crate::special_advance::SpecialAdvance;
        let [event] = self.events.as_slice() else {
            return None;
        };
        let handler = event.player.handler.as_ref()?;
        (matches!(self.state, GameState::Playing)
            && matches!(event.event_type, PersistentEventType::TurnStart)
            && event.origin_override.is_none()
            && event.players_used.is_empty()
            && handler.priority == 5
            && self.dropped_players.contains(&event.player.index)
            && handler.origin == EventOrigin::SpecialAdvance(SpecialAdvance::Banking)
            && matches!(handler.request, PersistentEventRequest::ResourceReward(_))
            && handler.response.is_none())
        .then_some(event.player.index)
    }

    /// Repair the old withdrawal shape: the seat pointer advanced, but Banking
    /// still belongs to the departed seat and the successor has not started.
    /// Removing the event before starting the successor makes repeated load/clone
    /// idempotent and avoids resuming `TurnStart` with the wrong seat pointer.
    pub(crate) fn reconcile_dropped_banking(&mut self) {
        let Some(player) = self.dropped_banking_player() else {
            return;
        };
        if self.human_players_count() <= 1 || self.current_player_index == player {
            return;
        }
        let seats = self.players.iter().filter(|p| p.is_human()).count();
        let next = (1..seats)
            .map(|offset| (player + offset) % seats)
            .find(|p| !self.dropped_players.contains(p));
        // The old code entered next_round at the boundary. Do not guess whether
        // that round's effects have already run; only repair the confirmed shape.
        if next != Some(self.current_player_index)
            || self.current_player_index == self.starting_player_index
            || self.log.is_empty()
            || current_turn_log(self).turn_type != TurnType::Player(player)
        {
            return;
        }
        self.events.clear();
        self.cleanup_withdrawn_turn(player);
        self.start_turn();
    }

    fn cleanup_withdrawn_turn(&mut self, player: usize) {
        end_turn(self, player);
        for action in &mut current_turn_log_mut(self).actions {
            action.undo.clear();
        }
        check_for_waste(self);
    }

    /// Remove a player while keeping surviving seat indices and round order stable.
    ///
    /// # Panics
    /// Panics if the saved human-seat layout is inconsistent with its active players.
    pub fn drop_player(&mut self, player_index: usize) {
        if self.civilization_draft.is_some() {
            crate::game_setup::drop_draft_player(self, player_index);
            return;
        }
        if self.dropped_players.contains(&player_index) {
            self.reconcile_dropped_banking();
            self.finish_after_drop();
            return;
        }
        self.dropped_players.push(player_index);
        if self.finish_after_drop() {
            return;
        }
        self.add_message(&format!(
            "{} has left the game",
            self.player_name(player_index)
        ));
        let dropped_first = self.starting_player_index == player_index;
        if dropped_first {
            let seats = self.players.iter().filter(|p| p.is_human()).count();
            self.starting_player_index = (1..seats)
                .map(|offset| (player_index + offset) % seats)
                .find(|p| !self.dropped_players.contains(p))
                .expect("an active player remains");
        }
        if self.current_player_index != player_index {
            return;
        }
        if self.dropped_banking_player() == Some(player_index) {
            self.events.clear();
            self.cleanup_withdrawn_turn(player_index);
        }
        self.skip_dropped_players();
        if !dropped_first && self.current_player_index == self.starting_player_index {
            self.next_round();
        } else if self.events.is_empty() {
            self.start_turn();
        }
    }

    #[must_use]
    pub fn available_custom_actions(&self, player_index: usize) -> Vec<SpecialActionInfo> {
        self.player(player_index)
            .special_actions
            .values()
            .filter(|&c| {
                if matches!(c.execution, SpecialActionExecution::Modifier(_)) {
                    // returned as part of "base_or_modified_available"
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

    #[must_use]
    pub fn is_update_patch(&self) -> bool {
        matches!(self.options.patch, PatchOption::BalancePatch)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    ChooseCivilization,
    Playing,
    Movement(MoveState),
    Finished,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UIElement {
    text: String,
    tooltip: String,
    priority: u8,
}
