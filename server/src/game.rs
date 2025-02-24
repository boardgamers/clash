use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::vec;
use GameState::*;

use crate::advance::Advance;
use crate::barbarians::BarbariansEventState;
use crate::combat::{
    self, combat_loop, combat_round_end, end_combat, start_combat, take_combat, Combat,
    CombatDieRoll, COMBAT_DIE_SIDES,
};
use crate::consts::{ACTIONS, MOVEMENT_ACTIONS};
use crate::content::civilizations::BARBARIANS;
use crate::content::custom_phase_actions::{CurrentCustomPhaseEvent, CustomPhaseEventState};
use crate::content::incidents;
use crate::events::{Event, EventOrigin};
use crate::explore::{explore_resolution, move_to_unexplored_tile, undo_explore_resolution};
use crate::map::UnexploredBlock;
use crate::movement::{has_movable_units, terrain_movement_restriction};
use crate::payment::PaymentOptions;
use crate::player_events::{
    ActionInfo, AdvanceInfo, CustomPhaseEvent, CustomPhaseInfo, IncidentInfo, InfluenceCultureInfo,
    MoveInfo, PlayerCommandEvent, PlayerCommands, PlayerEvents,
};
use crate::resource::check_for_waste;
use crate::status_phase::StatusPhaseAction;
use crate::unit::{
    carried_units, get_current_move, MoveUnits, MovementRestriction, UnitData, Units,
};
use crate::utils::Rng;
use crate::utils::Shuffle;
use crate::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::Building::{self, *},
    consts::AGES,
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    log::{self},
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
        UnitType::{self},
    },
    utils,
    wonder::Wonder,
};

pub struct Game {
    pub state: GameState,
    pub custom_phase_state: Vec<CustomPhaseEventState>,
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
    pub wonder_amount_left: usize,
    pub incidents_left: Vec<u8>,
    pub undo_context_stack: Vec<UndoContext>, // transient
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
            // exclude barbarians
            let civilization = rng.range(1, civilizations.len());
            players.push(Player::new(civilizations.remove(civilization), i));
        }

        let starting_player = rng.range(0, players.len());

        players.push(Player::new(
            civilizations::get_civilization(BARBARIANS).expect("civ not found"),
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

        Self {
            state: Playing,
            custom_phase_state: Vec::new(),
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
            round: data.round,
            age: data.age,
            messages: data.messages,
            rng: data.rng,
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
            custom_phase_state: data.state_change_event_state,
            undo_context_stack: Vec::new(),
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
            state_change_event_state: self.custom_phase_state,
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
            rng: self.rng,
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
        }
    }

    #[must_use]
    pub fn cloned_data(&self) -> GameData {
        GameData {
            state: self.state.clone(),
            state_change_event_state: self.custom_phase_state.clone(),
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
            rng: self.rng.clone(),
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
        }
    }

    #[must_use]
    pub fn get_player(&self, player_index: usize) -> &Player {
        &self.players[player_index]
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

    pub(crate) fn add_action_log_item(&mut self, item: Action) {
        if self.action_log_index < self.action_log.len() {
            self.action_log.drain(self.action_log_index..);
        }
        self.action_log.push(ActionLogItem::new(item));
        self.action_log_index += 1;
    }

    pub(crate) fn lock_undo(&mut self) {
        self.undo_limit = self.action_log_index;
    }

    #[must_use]
    pub(crate) fn current_custom_phase(&self) -> &CustomPhaseEventState {
        self.custom_phase_state.last().expect("state should exist")
    }

    #[must_use]
    pub(crate) fn current_custom_phase_mut(&mut self) -> &mut CustomPhaseEventState {
        self.custom_phase_state
            .last_mut()
            .expect("state should exist")
    }

    #[must_use]
    pub fn current_custom_phase_event(&self) -> Option<&CurrentCustomPhaseEvent> {
        self.custom_phase_state
            .last()
            .and_then(|s| s.current.as_ref())
    }

    pub fn current_custom_phase_event_mut(&mut self) -> Option<&mut CurrentCustomPhaseEvent> {
        self.custom_phase_state
            .last_mut()
            .and_then(|s| s.current.as_mut())
    }

    pub(crate) fn trigger_custom_phase_event<V>(
        &mut self,
        players: &[usize],
        event: fn(&mut PlayerEvents) -> &mut CustomPhaseEvent<V>,
        details: &V,
        log: Option<&str>,
    ) -> bool {
        let name = event(&mut self.players[0].events).name.clone();
        if self
            .custom_phase_state
            .last()
            .is_none_or(|s| s.event_type != name)
        {
            if let Some(log) = log {
                self.add_info_log_group(log.to_string());
            }
            self.custom_phase_state
                .push(CustomPhaseEventState::new(name));
        }
        let state = self.current_custom_phase();

        let remaining: Vec<_> = players
            .iter()
            .filter(|&p| !state.players_used.contains(p))
            .collect();

        for &player_index in remaining {
            let info = CustomPhaseInfo {
                player: player_index,
            };
            self.trigger_event_with_game_value(player_index, event, &info, details);
            if self.current_custom_phase().current.is_some() {
                return true;
            }
            let state = self.current_custom_phase_mut();
            state.players_used.push(player_index);
            state.last_priority_used = None;
        }
        self.custom_phase_state.pop();
        false
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

    pub(crate) fn trigger_command_event<V>(
        &mut self,
        player_index: usize,
        event: fn(&mut PlayerEvents) -> &mut PlayerCommandEvent<V>,
        details: &V,
    ) {
        let e = event(&mut self.players[player_index].events).take();
        self.with_commands(player_index, |commands, game| {
            let _ = e.trigger(commands, game, details);
        });
        event(&mut self.players[player_index].events).set(e);
    }

    pub(crate) fn with_commands(
        &mut self,
        player_index: usize,
        callback: impl FnOnce(&mut PlayerCommands, &mut Game),
    ) {
        let p = self.get_player(player_index);
        let info = CommandUndoInfo::new(p);
        let mut commands = PlayerCommands::new(player_index, p.get_name(), p.event_info.clone());

        callback(&mut commands, self);

        info.apply(self, commands.content.clone());
        self.players[player_index].gain_resources(commands.content.gained_resources);

        for edit in commands.log {
            self.add_info_log_item(&edit);
        }
        for u in commands.content.gained_units {
            self.players[u.player].add_unit(u.position, u.unit_type);
        }
        for city in commands.content.gained_cities {
            self.players[city.player]
                .cities
                .push(City::new(city.player, city.position));
        }
        if let Some(mut p) = commands.content.barbarian_update {
            if let Some(m) = p.move_request.take() {
                let from = m.from;
                let to = m.to;
                let vec = self.get_player(m.player).get_units(from);
                let units: Vec<u32> = vec.iter().map(|u| u.id).collect();
                let unit_types = vec.iter().map(|u| u.unit_type).collect::<Units>();
                self.add_info_log_item(&format!(
                    "Barbarians move from {from} to {to}: {unit_types}"
                ));
                p.moved_units.extend(units.iter());
                self.move_with_possible_combat(
                    m.player,
                    None,
                    from,
                    &MoveUnits {
                        units,
                        destination: to,
                        embark_carrier_id: None,
                        payment: ResourcePile::empty(),
                    },
                );
            }
            self.current_custom_phase_mut().barbarians = Some(p);
        }
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

        if matches!(action, Action::Redo) {
            assert!(self.can_redo(), "no action can be redone");
            self.redo(player_index);
            return;
        }

        self.add_log_item_from_action(&action);
        self.add_action_log_item(action.clone());

        if let Some(s) = &mut self.current_custom_phase_event_mut() {
            s.response = action.custom_phase_event();
            let event_type = self.current_custom_phase().event_type.clone();
            self.execute_custom_phase_action(player_index, &event_type);
        } else {
            self.execute_regular_action(action, player_index);
        }
        check_for_waste(self);

        self.action_log[self.action_log_index - 1].undo =
            std::mem::take(&mut self.undo_context_stack);
    }

    pub(crate) fn execute_custom_phase_action(&mut self, player_index: usize, event_type: &str) {
        match event_type {
            "on_combat_start" => {
                start_combat(self);
            }
            "on_combat_round_end" => {
                self.lock_undo();
                if let Some(c) = combat_round_end(self) {
                    combat_loop(self, c);
                }
            }
            "on_combat_end" => {
                let c = take_combat(self);
                end_combat(self, c);
            }
            "on_turn_start" => self.start_turn(),
            // name and payment is ignored here
            "on_advance_custom_phase" => {
                self.on_advance(player_index, ResourcePile::empty(), "");
            }
            "on_construct" => {
                // building is ignored here
                PlayingAction::on_construct(self, player_index, Temple);
            }
            "on_recruit" => {
                self.on_recruit(player_index);
            }
            "on_incident" => {
                self.trigger_incident(player_index);
            }
            _ => panic!("unknown custom phase event {event_type}"),
        }
    }

    fn add_log_item_from_action(&mut self, action: &Action) {
        self.log.push(log::format_action_log_item(action, self));
    }

    fn execute_regular_action(&mut self, action: Action, player_index: usize) {
        match self.state.clone() {
            Playing => {
                if let Some(m) = action.clone().movement() {
                    self.execute_movement_action(m, player_index, MoveState::new());
                } else {
                    let action = action.playing().expect("action should be a playing action");

                    action.execute(self, player_index);
                }
            }
            StatusPhase(phase) => {
                let action = action
                    .status_phase()
                    .expect("action should be a status phase action");
                assert!(phase == action.phase(), "Illegal action: Same phase again");
                action.execute(self, player_index);
            }
            Movement(m) => {
                let action = action
                    .movement()
                    .expect("action should be a movement action");
                self.execute_movement_action(action, player_index, m);
            }
            CulturalInfluenceResolution(c) => {
                let action = action
                    .cultural_influence_resolution()
                    .expect("action should be a cultural influence resolution action");
                self.execute_cultural_influence_resolution_action(
                    action,
                    c.roll_boost_cost,
                    c.target_player_index,
                    c.target_city_position,
                    c.city_piece,
                    player_index,
                );
            }
            Combat(_) => {
                panic!("actions can't be executed when the game is in a combat state");
            }
            ExploreResolution(r) => {
                let rotation = action
                    .explore_resolution()
                    .expect("action should be an explore resolution action");
                explore_resolution(self, &r, rotation);
            }
            Finished => panic!("actions can't be executed when the game is finished"),
        }
    }

    fn undo(&mut self, player_index: usize) {
        self.action_log_index -= 1;
        self.log.remove(self.log.len() - 1);
        let item = &self.action_log[self.action_log_index];
        self.undo_context_stack = item.undo.clone();
        let action = item.action.clone();

        let was_custom_phase = self.current_custom_phase_event().is_some();
        if was_custom_phase {
            self.custom_phase_state.pop();
        }

        match action {
            Action::Playing(action) => action.clone().undo(self, player_index, was_custom_phase),
            Action::StatusPhase(_) => panic!("status phase actions can't be undone"),
            Action::Movement(action) => {
                self.undo_movement_action(action.clone(), player_index);
            }
            Action::CulturalInfluenceResolution(action) => {
                self.undo_cultural_influence_resolution_action(action);
            }
            Action::ExploreResolution(_rotation) => {
                undo_explore_resolution(self, player_index);
            }
            Action::CustomPhaseEvent(action) => action.clone().undo(self, player_index),
            Action::Undo => panic!("undo action can't be undone"),
            Action::Redo => panic!("redo action can't be undone"),
        }

        if let Some(UndoContext::WastedResources {
            resources,
            player_index,
        }) = self.maybe_pop_undo_context(|c| matches!(c, UndoContext::WastedResources { .. }))
        {
            self.players[player_index].gain_resources_in_undo(resources.clone());
        }

        while self.maybe_pop_undo_context(|_| false).is_some() {
            // pop all undo contexts until action start
        }
    }

    fn redo(&mut self, player_index: usize) {
        let copy = self.action_log[self.action_log_index].clone();
        self.add_log_item_from_action(&copy.action);
        match &self.action_log[self.action_log_index].action {
            Action::Playing(action) => action.clone().execute(self, player_index),
            Action::StatusPhase(_) => panic!("status phase actions can't be redone"),
            Action::Movement(action) => match &self.state {
                Playing => {
                    self.execute_movement_action(action.clone(), player_index, MoveState::new());
                }
                Movement(m) => {
                    self.execute_movement_action(action.clone(), player_index, m.clone());
                }
                _ => {
                    panic!("movement actions can only be redone if the game is in a movement state")
                }
            },
            Action::CulturalInfluenceResolution(action) => {
                let CulturalInfluenceResolution(c) = &self.state else {
                    panic!("cultural influence resolution actions can only be redone if the game is in a cultural influence resolution state");
                };
                self.execute_cultural_influence_resolution_action(
                    *action,
                    c.roll_boost_cost.clone(),
                    c.target_player_index,
                    c.target_city_position,
                    c.city_piece,
                    player_index,
                );
            }
            Action::ExploreResolution(rotation) => {
                let ExploreResolution(r) = &self.state else {
                    panic!("explore resolution actions can only be redone if the game is in a explore resolution state");
                };
                explore_resolution(self, &r.clone(), *rotation);
            }
            Action::CustomPhaseEvent(action) => action.clone().redo(self, player_index),
            Action::Undo => panic!("undo action can't be redone"),
            Action::Redo => panic!("redo action can't be redone"),
        }
        self.action_log_index += 1;
        check_for_waste(self);
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
        mut move_state: MoveState,
    ) {
        let saved_state = move_state.clone();
        let (starting_position, disembarked_units) = match action {
            Move(m) => {
                if let Playing = self.state {
                    assert_ne!(self.actions_left, 0, "Illegal action");
                    self.actions_left -= 1;
                }
                let player = &self.players[player_index];
                let starting_position = player
                    .get_unit(*m.units.first().expect(
                        "instead of providing no units to move a stop movement actions should be done",
                    ))
                    .expect("the player should have all units to move")
                    .position;
                let disembarked_units = m
                    .units
                    .iter()
                    .filter_map(|unit| {
                        let unit = player.get_unit(*unit).expect("unit should exist");
                        unit.carrier_id.map(|carrier_id| DisembarkUndoContext {
                            unit_id: unit.id,
                            carrier_id,
                        })
                    })
                    .collect();
                match player.move_units_destinations(
                    self,
                    &m.units,
                    starting_position,
                    m.embark_carrier_id,
                ) {
                    Ok(destinations) => {
                        let c = &destinations
                            .iter()
                            .find(|route| route.destination == m.destination)
                            .expect("destination should be a valid destination")
                            .cost;
                        if c.is_free() {
                            assert_eq!(m.payment, ResourcePile::empty(), "payment should be empty");
                        } else {
                            self.players[player_index].pay_cost(c, &m.payment);
                        }
                    }
                    Err(e) => {
                        panic!("cannot move units to destination: {e}");
                    }
                }

                move_state.moved_units.extend(m.units.iter());
                move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();
                let current_move = get_current_move(
                    self,
                    &m.units,
                    starting_position,
                    m.destination,
                    m.embark_carrier_id,
                );
                if matches!(current_move, CurrentMove::None)
                    || move_state.current_move != current_move
                {
                    move_state.movement_actions_left -= 1;
                    move_state.current_move = current_move;
                }

                let dest_terrain = self
                    .map
                    .get(m.destination)
                    .expect("destination should be a valid tile");

                if dest_terrain == &Unexplored {
                    if move_to_unexplored_tile(
                        self,
                        player_index,
                        &m.units,
                        starting_position,
                        m.destination,
                        &move_state,
                    ) {
                        self.back_to_move(&move_state, true);
                    }
                    return;
                }

                if self.move_with_possible_combat(
                    player_index,
                    Some(&mut move_state),
                    starting_position,
                    &m,
                ) {
                    return;
                }

                (Some(starting_position), disembarked_units)
            }
            Stop => {
                self.state = Playing;
                (None, Vec::new())
            }
        };
        self.push_undo_context(UndoContext::Movement {
            starting_position,
            move_state: saved_state,
            disembarked_units,
        });
    }

    fn move_with_possible_combat(
        &mut self,
        player_index: usize,
        move_state: Option<&mut MoveState>,
        starting_position: Position,
        m: &MoveUnits,
    ) -> bool {
        let enemy = self.enemy_player(player_index, m.destination);
        if let Some(defender) = enemy {
            if self.move_to_defended_tile(
                player_index,
                move_state,
                &m.units,
                m.destination,
                starting_position,
                defender,
            ) {
                return true;
            }
        } else {
            self.move_units(player_index, &m.units, m.destination, m.embark_carrier_id);
            if let Some(move_state) = move_state {
                self.back_to_move(move_state, !starting_position.is_neighbor(m.destination));
            }
        }

        if let Some(enemy) = enemy {
            self.capture_position(enemy, m.destination, player_index);
        }
        false
    }

    pub(crate) fn push_undo_context(&mut self, context: UndoContext) {
        self.undo_context_stack.push(context);
    }

    pub(crate) fn pop_undo_context(&mut self) -> Option<UndoContext> {
        self.maybe_pop_undo_context(|_| true)
    }

    pub(crate) fn maybe_pop_undo_context(
        &mut self,
        pred: fn(&UndoContext) -> bool,
    ) -> Option<UndoContext> {
        loop {
            if let Some(context) = &self.undo_context_stack.last() {
                if let UndoContext::Command(_) = context {
                    let Some(UndoContext::Command(c)) = self.undo_context_stack.pop() else {
                        panic!("when popping a command undo context, the undo context stack should have a command undo context")
                    };
                    self.undo_commands(&c);
                } else {
                    if pred(context) {
                        return self.undo_context_stack.pop();
                    }
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    fn undo_commands(&mut self, c: &CommandContext) {
        let p = self.current_player_index;
        self.players[p].event_info.clone_from(&c.info);
        self.players[p].lose_resources(c.gained_resources.clone());
        for u in &c.gained_units {
            self.undo_recruit_without_activate(u.player, &[u.unit_type], None);
        }
        // gained_cities is only for Barbarians
    }

    fn move_to_defended_tile(
        &mut self,
        player_index: usize,
        move_state: Option<&mut MoveState>,
        units: &Vec<u32>,
        destination: Position,
        starting_position: Position,
        defender: usize,
    ) -> bool {
        let has_defending_units = self.players[defender]
            .get_units(destination)
            .iter()
            .any(|unit| !unit.unit_type.is_settler());
        let has_fortress = self.players[defender]
            .get_city(destination)
            .is_some_and(|city| city.pieces.fortress.is_some());

        let mut military = false;
        for unit_id in units {
            let unit = self.players[player_index]
                .get_unit_mut(*unit_id)
                .expect("the player should have all units to move");
            if !unit.unit_type.is_settler() {
                if unit
                    .movement_restrictions
                    .contains(&MovementRestriction::Battle)
                {
                    panic!("unit can't attack");
                }
                unit.movement_restrictions.push(MovementRestriction::Battle);
                military = true;
            }
        }
        assert!(military, "Need military units to attack");
        if let Some(move_state) = move_state {
            self.back_to_move(move_state, true);
        }

        if has_defending_units || has_fortress {
            combat::initiate_combat(
                self,
                defender,
                destination,
                player_index,
                starting_position,
                units.clone(),
                self.get_player(player_index).is_human(),
                None,
            );
            return true;
        }
        false
    }

    fn undo_movement_action(&mut self, action: MovementAction, player_index: usize) {
        let Some(UndoContext::Movement {
            starting_position,
            move_state,
            disembarked_units,
        }) = self.pop_undo_context()
        else {
            panic!("when undoing a movement action, the game should have stored movement context")
        };
        if let Move(m) = action {
            self.undo_move_units(
                player_index,
                m.units,
                starting_position.expect(
                    "undo context should contain the starting position if units where moved",
                ),
            );
            self.players[player_index].gain_resources_in_undo(m.payment);
            for unit in disembarked_units {
                self.players[player_index]
                    .get_unit_mut(unit.unit_id)
                    .expect("unit should exist")
                    .carrier_id = Some(unit.carrier_id);
            }
        }
        if move_state.movement_actions_left == MOVEMENT_ACTIONS {
            self.state = Playing;
            self.actions_left += 1;
        } else {
            self.state = Movement(move_state);
        }
    }

    fn execute_cultural_influence_resolution_action(
        &mut self,
        action: bool,
        roll_boost_cost: ResourcePile,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
        player_index: usize,
    ) {
        self.state = Playing;
        if !action {
            return;
        }
        self.players[player_index].lose_resources(roll_boost_cost.clone());
        self.push_undo_context(UndoContext::InfluenceCultureResolution { roll_boost_cost });
        self.influence_culture(
            player_index,
            target_player_index,
            target_city_position,
            city_piece,
        );
    }

    fn undo_cultural_influence_resolution_action(&mut self, action: bool) {
        let cultural_influence_attempt_action = self.action_log[self.action_log_index - 1].action.playing_ref().expect("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        let PlayingAction::InfluenceCultureAttempt(c) = cultural_influence_attempt_action else {
            panic!("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        };

        let city_piece = c.city_piece;
        let target_player_index = c.target_player_index;
        let target_city_position = c.target_city_position;

        let Some(UndoContext::InfluenceCultureResolution { roll_boost_cost }) =
            self.pop_undo_context()
        else {
            panic!("when undoing a cultural influence resolution action, the game should have stored influence culture resolution context")
        };

        self.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
            roll_boost_cost: roll_boost_cost.clone(),
            target_player_index,
            target_city_position,
            city_piece,
        });
        if !action {
            return;
        }
        self.players[self.current_player_index].gain_resources_in_undo(roll_boost_cost);
        self.undo_influence_culture(target_player_index, target_city_position, city_piece);
    }

    pub(crate) fn back_to_move(&mut self, move_state: &MoveState, stop_current_move: bool) {
        let mut state = move_state.clone();
        if stop_current_move {
            state.current_move = CurrentMove::None;
        }
        // set state to Movement first, because that affects has_movable_units
        self.state = Movement(state);

        let all_moves_used =
            move_state.movement_actions_left == 0 && move_state.current_move == CurrentMove::None;
        if all_moves_used || !has_movable_units(self, self.get_player(self.current_player_index)) {
            self.state = Playing;
        }
    }

    #[must_use]
    pub fn enemy_player(&self, player_index: usize, position: Position) -> Option<usize> {
        self.players.iter().position(|player| {
            player.index != player_index
                && (!player.get_units(position).is_empty() || player.get_city(position).is_some())
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
            self.players[self.current_player_index].get_name()
        ));
        self.lock_undo();

        self.start_turn();
    }

    fn start_turn(&mut self) {
        self.trigger_custom_phase_event(
            &[self.current_player_index],
            |e| &mut e.on_turn_start,
            &(),
            None,
        );
    }

    pub fn skip_dropped_players(&mut self) {
        if self.human_players().is_empty() {
            return;
        }
        while self.dropped_players.contains(&self.current_player_index)
            && self.current_player_index != self.starting_player_index
        {
            self.increment_player_index();
        }
    }

    pub fn increment_player_index(&mut self) {
        // Barbarians have the highest player index
        self.current_player_index += 1;
        self.current_player_index %= self.human_players().len();
    }

    #[must_use]
    pub fn active_player(&self) -> usize {
        if let Some(custom_phase_event) = &self.current_custom_phase_event() {
            return custom_phase_event.player_index;
        }
        self.current_player_index
    }

    #[must_use]
    pub fn human_players(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if p.civilization.is_human() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn next_turn(&mut self) {
        self.actions_left = ACTIONS;
        self.successful_cultural_influence = false;
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
        self.add_info_log_group(format!("Round {}/3", self.round));
    }

    fn enter_status_phase(&mut self) {
        if self
            .players
            .iter()
            .filter(|player| player.is_human())
            .any(|player| player.cities.is_empty())
        {
            self.end_game();
        }
        self.add_info_log_group(format!(
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
        self.add_info_log_group(format!("Age {} has started", self.age));
        self.add_info_log_group(String::from("Round 1/3"));
    }

    fn end_game(&mut self) {
        self.state = Finished;
        let winner_player_index = self
            .players
            .iter()
            .enumerate()
            .max_by(|(_, player), (_, other)| player.compare_score(other, self))
            .expect("there should be at least one player in the game")
            .0;
        let winner_name = self.players[winner_player_index].get_name();
        self.add_info_log_group(format!("The game has ended. {winner_name} has won"));
        self.add_message("The game has ended");
    }

    pub(crate) fn get_next_dice_roll(&mut self) -> CombatDieRoll {
        self.lock_undo();
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

    pub fn draw_wonder_card(&mut self, player_index: usize) {
        let Some(wonder) = self.wonders_left.pop() else {
            return;
        };

        self.wonder_amount_left -= 1;
        self.players[player_index].wonder_cards.push(wonder);
        self.lock_undo();
    }

    fn set_active_leader(&mut self, leader_name: String, player_index: usize) {
        self.players[player_index]
            .available_leaders
            .retain(|name| name != &leader_name);
        Player::with_leader(&leader_name, self, player_index, |game, leader| {
            (leader.listeners.initializer)(game, player_index);
            (leader.listeners.one_time_initializer)(game, player_index);
        });
        self.players[player_index].active_leader = Some(leader_name);
    }

    pub(crate) fn advance_with_incident_token(
        &mut self,
        advance: &str,
        player_index: usize,
        payment: ResourcePile,
    ) {
        self.advance(advance, player_index);
        self.on_advance(player_index, payment, advance);
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if advance does not exist
    pub fn advance(&mut self, advance: &str, player_index: usize) {
        self.trigger_command_event(player_index, |e| &mut e.on_advance, &advance.to_string());
        let advance = advances::get_advance(advance);
        (advance.listeners.initializer)(self, player_index);
        (advance.listeners.one_time_initializer)(self, player_index);
        let name = advance.name.clone();
        for i in 0..self.players[player_index]
            .civilization
            .special_advances
            .len()
        {
            if self.players[player_index].civilization.special_advances[i].required_advance == name
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
        if let Some(advance_bonus) = &advance.bonus {
            let pile = advance_bonus.resources();
            self.add_info_log_item(&format!("Player gained {pile} as advance bonus"));
            self.players[player_index].gain_resources(pile);
        }
        let player = &mut self.players[player_index];
        player.advances.push(advance);
    }

    pub(crate) fn on_advance(&mut self, player_index: usize, payment: ResourcePile, advance: &str) {
        if self.trigger_custom_phase_event(
            &[player_index],
            |e| &mut e.on_advance_custom_phase,
            &AdvanceInfo {
                name: advance.to_string(),
                payment,
            },
            None,
        ) {
            return;
        }
        let player = &mut self.players[player_index];
        player.incident_tokens -= 1;
        if player.incident_tokens == 0 {
            player.incident_tokens = 3;
            self.trigger_incident(player_index);
        }
    }

    pub(crate) fn undo_advance(
        &mut self,
        advance: &Advance,
        player_index: usize,
        was_custom_phase: bool,
    ) {
        self.remove_advance(advance, player_index);
        if !was_custom_phase {
            self.players[player_index].incident_tokens += 1;
        }
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
        units: Units,
        city_position: Position,
        leader_name: Option<&String>,
        replaced_units: &[u32],
    ) {
        let mut replaced_leader = None;
        if let Some(leader_name) = leader_name {
            if let Some(previous_leader) = self.players[player_index].active_leader.take() {
                Player::with_leader(
                    &previous_leader,
                    self,
                    player_index,
                    |game, previous_leader| {
                        (previous_leader.listeners.deinitializer)(game, player_index);
                    },
                );
                replaced_leader = Some(previous_leader);
            }
            self.set_active_leader(leader_name.clone(), player_index);
        }
        let mut replaced_units_undo_context = Vec::new();
        for unit in replaced_units {
            let player = &mut self.players[player_index];
            let u = player.remove_unit(*unit);
            if u.carrier_id.is_some_and(|c| replaced_units.contains(&c)) {
                // will be removed when the carrier is removed
                continue;
            }
            let unit = u.data(self.get_player(player_index));
            replaced_units_undo_context.push(unit);
        }
        self.push_undo_context(UndoContext::Recruit {
            replaced_units: replaced_units_undo_context,
            replaced_leader,
        });
        let player = &mut self.players[player_index];
        let vec = units.to_vec();
        player.units.reserve_exact(vec.len());
        for unit_type in vec {
            let city = player
                .get_city(city_position)
                .expect("player should have a city at the recruitment position");
            let position = match &unit_type {
                UnitType::Ship => city
                    .port_position
                    .expect("there should be a port in the city"),
                _ => city_position,
            };
            player.add_unit(position, unit_type);
        }
        let city = player
            .get_city_mut(city_position)
            .expect("player should have a city at the recruitment position");
        city.activate();
        self.on_recruit(player_index);
    }

    fn find_last_action(&self, pred: fn(&Action) -> bool) -> Option<Action> {
        self.action_log
            .iter()
            .rev()
            .find(|item| pred(&item.action))
            .map(|item| item.action.clone())
    }

    fn on_recruit(&mut self, player_index: usize) {
        let Some(Action::Playing(PlayingAction::Recruit(r))) = self.find_last_action(|action| {
            matches!(action, Action::Playing(PlayingAction::Recruit(_)))
        }) else {
            panic!("last action should be a recruit action")
        };

        if self.trigger_custom_phase_event(
            &[player_index],
            |events| &mut events.on_recruit,
            &r,
            None,
        ) {
            return;
        }
        let city_position = r.city_position;

        if let Some(port_position) = self.players[player_index]
            .get_city(city_position)
            .and_then(|city| city.port_position)
        {
            let ships = self.players[player_index]
                .get_units(port_position)
                .iter()
                .filter(|unit| unit.unit_type.is_ship())
                .map(|unit| unit.id)
                .collect::<Vec<_>>();
            if !ships.is_empty() {
                if let Some(defender) = self.enemy_player(player_index, port_position) {
                    for ship in self.players[player_index].get_units_mut(port_position) {
                        ship.position = city_position;
                    }
                    combat::initiate_combat(
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
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn undo_recruit(
        &mut self,
        player_index: usize,
        units: Units,
        city_position: Position,
        leader_name: Option<&String>,
    ) {
        self.undo_recruit_without_activate(player_index, &units.to_vec(), leader_name);
        self.players[player_index]
            .get_city_mut(city_position)
            .expect("player should have a city a recruitment position")
            .undo_activate();
        if let Some(UndoContext::Recruit {
            replaced_units,
            replaced_leader,
        }) = self.pop_undo_context()
        {
            let player = &mut self.players[player_index];
            for unit in replaced_units {
                player.units.extend(Unit::from_data(player_index, unit));
            }
            if let Some(replaced_leader) = replaced_leader {
                player.active_leader = Some(replaced_leader.clone());
                Player::with_leader(
                    &replaced_leader,
                    self,
                    player_index,
                    |game, replaced_leader| {
                        (replaced_leader.listeners.initializer)(game, player_index);
                        (replaced_leader.listeners.one_time_initializer)(game, player_index);
                    },
                );
            }
        }
    }

    fn undo_recruit_without_activate(
        &mut self,
        player_index: usize,
        units: &[UnitType],
        leader_name: Option<&String>,
    ) {
        if let Some(leader_name) = leader_name {
            let current_leader = self.players[player_index]
                .active_leader
                .take()
                .expect("the player should have an active leader");
            Player::with_leader(
                &current_leader,
                self,
                player_index,
                |game, current_leader| {
                    (current_leader.listeners.deinitializer)(game, player_index);
                    (current_leader.listeners.undo_deinitializer)(game, player_index);
                },
            );

            self.players[player_index]
                .available_leaders
                .push(leader_name.clone());
            self.players[player_index].available_leaders.sort();

            self.players[player_index].active_leader = None;
        }
        let player = &mut self.players[player_index];
        for _ in 0..units.len() {
            player
                .units
                .pop()
                .expect("the player should have the recruited units when undoing");
            player.next_unit_id -= 1;
        }
    }

    fn trigger_incident(&mut self, player_index: usize) {
        self.lock_undo();

        if self.incidents_left.is_empty() {
            self.incidents_left = incidents::get_all().iter().map(|i| i.id).collect_vec();
            self.incidents_left.shuffle(&mut self.rng);
        }

        let id = *self.incidents_left.first().expect("incident should exist");
        for p in &self.human_players() {
            (incidents::get_incident(id).listeners.initializer)(self, *p);
        }

        let i = self
            .human_players()
            .iter()
            .position(|&p| p == player_index)
            .expect("player should exist");
        let mut players: Vec<_> = self.human_players();
        players.rotate_left(i);

        self.trigger_custom_phase_event(
            &players,
            |events| &mut events.on_incident,
            &IncidentInfo::new(player_index),
            Some("A new game event has been triggered: "),
        );

        for p in &players {
            (incidents::get_incident(id).listeners.deinitializer)(self, *p);
        }

        if self.custom_phase_state.is_empty() {
            self.incidents_left.remove(0);

            if matches!(self.state, GameState::StatusPhase(_)) {
                StatusPhaseAction::action_done(self);
            }
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if advance does not exist
    pub fn remove_advance(&mut self, advance: &Advance, player_index: usize) {
        (advance.listeners.deinitializer)(self, player_index);
        (advance.listeners.undo_deinitializer)(self, player_index);

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
        if let Some(advance_bonus) = &advance.bonus {
            player.lose_resources(advance_bonus.resources());
        }
        utils::remove_element(&mut self.players[player_index].advances, advance);
    }

    fn unlock_special_advance(&mut self, special_advance: &SpecialAdvance, player_index: usize) {
        (special_advance.listeners.initializer)(self, player_index);
        (special_advance.listeners.one_time_initializer)(self, player_index);
        self.players[player_index]
            .unlocked_special_advances
            .push(special_advance.name.clone());
    }

    fn undo_unlock_special_advance(
        &mut self,
        special_advance: &SpecialAdvance,
        player_index: usize,
    ) {
        (special_advance.listeners.deinitializer)(self, player_index);
        (special_advance.listeners.undo_deinitializer)(self, player_index);
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
        let Some(mut city) = self.players[old_player_index].take_city(position) else {
            panic!("player should have this city")
        };
        // undo would only be possible if the old owner can't spawn a settler
        // and this would be hard to understand
        self.lock_undo();
        self.add_to_last_log_item(&format!(
            " and captured {}'s city at {position}",
            self.players[old_player_index].get_name()
        ));
        let attacker_is_human = self.get_player(new_player_index).is_human();
        let size = city.mood_modified_size(&self.players[new_player_index]);
        if attacker_is_human {
            self.players[new_player_index].gain_resources(ResourcePile::gold(size as u32));
        }
        let take_over = self.get_player(new_player_index).is_city_available();

        if take_over {
            city.player_index = new_player_index;
            city.mood_state = Angry;
            if attacker_is_human {
                for wonder in &city.pieces.wonders {
                    (wonder.listeners.deinitializer)(self, old_player_index);
                    (wonder.listeners.initializer)(self, new_player_index);
                }

                for (building, owner) in city.pieces.building_owners() {
                    if matches!(building, Obelisk) {
                        continue;
                    }
                    let Some(owner) = owner else {
                        continue;
                    };
                    if owner != old_player_index {
                        continue;
                    }
                    if self.players[new_player_index].is_building_available(building, self) {
                        city.pieces.set_building(building, new_player_index);
                    } else {
                        city.pieces.remove_building(building);
                        self.players[new_player_index].gain_resources(ResourcePile::gold(1));
                    }
                }
            }
            self.players[new_player_index].cities.push(city);
        } else {
            self.players[new_player_index].gain_resources(ResourcePile::gold(city.size() as u32));
            city.raze(self, old_player_index);
        }
    }

    pub fn capture_position(&mut self, old_player: usize, position: Position, new_player: usize) {
        let captured_settlers = self.players[old_player]
            .get_units(position)
            .iter()
            .map(|unit| unit.id)
            .collect_vec();
        if !captured_settlers.is_empty() {
            self.add_to_last_log_item(&format!(
                " and killed {} settlers of {}",
                captured_settlers.len(),
                self.players[old_player].get_name()
            ));
        }
        for id in captured_settlers {
            self.players[old_player].remove_unit(id);
        }
        if self.get_player(old_player).get_city(position).is_some() {
            self.conquer_city(position, new_player, old_player);
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
        player.wonders_build.pop();
        let wonder = player
            .get_city_mut(city_position)
            .expect("player should have city")
            .pieces
            .wonders
            .pop()
            .expect("city should have a wonder");
        (wonder.listeners.deinitializer)(self, player_index);
        (wonder.listeners.undo_deinitializer)(self, player_index);
        wonder
    }

    fn influence_distance(
        &self,
        src: Position,
        dst: Position,
        visited: &[Position],
        len: u32,
    ) -> u32 {
        if visited.contains(&src) {
            return u32::MAX;
        }
        let mut visited = visited.to_vec();
        visited.push(src);

        if src == dst {
            return len;
        }
        src.neighbors()
            .into_iter()
            .filter(|&p| self.map.is_water(p) || self.map.is_land(p))
            .map(|n| self.influence_distance(n, dst, &visited, len + 1))
            .min()
            .expect("there should be a path")
    }

    #[must_use]
    pub fn influence_culture_boost_cost(
        &self,
        player_index: usize,
        starting_city_position: Position,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
    ) -> InfluenceCultureInfo {
        //todo allow cultural influence of barbarians
        let starting_city = self.get_city(player_index, starting_city_position);

        let range_boost = self
            .influence_distance(starting_city_position, target_city_position, &[], 0)
            .saturating_sub(starting_city.size() as u32);

        let self_influence = starting_city_position == target_city_position;
        let target_city = self.get_city(target_player_index, target_city_position);
        let target_city_owner = target_city.player_index;
        let target_building_owner = target_city.pieces.building_owner(city_piece);
        let attacker = &self.players[player_index];
        let defender = &self.players[target_player_index];
        let start_city_is_eligible = !starting_city.influenced() || self_influence;

        let mut info = InfluenceCultureInfo::new(
            PaymentOptions::resources(ResourcePile::culture_tokens(range_boost)),
            ActionInfo::new(attacker),
        );
        let _ = attacker.events.on_influence_culture_attempt.get().trigger(
            &mut info,
            target_city,
            self,
        );
        info.is_defender = true;
        let _ = defender.events.on_influence_culture_attempt.get().trigger(
            &mut info,
            target_city,
            self,
        );

        if !matches!(city_piece, Building::Obelisk)
            && starting_city.player_index == player_index
            && info.is_possible(range_boost)
            && attacker.can_afford(&info.range_boost_cost)
            && start_city_is_eligible
            && !self.successful_cultural_influence
            && attacker.is_building_available(city_piece, self)
            && target_city_owner == target_player_index
            && target_building_owner.is_some_and(|o| o != player_index)
        {
            return info;
        }
        info.set_impossible();
        info
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
        building: Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced player should have influenced city")
            .pieces
            .set_building(building, influencer_index);
        self.successful_cultural_influence = true;

        self.trigger_command_event(
            influencer_index,
            |e| &mut e.on_influence_culture_success,
            &(),
        );
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the influenced player does not have the influenced city
    pub fn undo_influence_culture(
        &mut self,
        influenced_player_index: usize,
        city_position: Position,
        building: Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced player should have influenced city")
            .pieces
            .set_building(building, influenced_player_index);
        self.successful_cultural_influence = false;
    }

    pub fn draw_new_cards(&mut self) {
        //todo every player draws 1 action card and 1 objective card
    }

    pub(crate) fn move_units(
        &mut self,
        player_index: usize,
        units: &[u32],
        to: Position,
        embark_carrier_id: Option<u32>,
    ) {
        let p = self.get_player(player_index);
        let from = p.get_unit(units[0]).expect("unit not found").position;
        let info = MoveInfo::new(player_index, units.to_vec(), from, to);
        self.trigger_command_event(player_index, |e| &mut e.before_move, &info);

        for unit_id in units {
            self.move_unit(player_index, *unit_id, to, embark_carrier_id);
        }
    }

    fn move_unit(
        &mut self,
        player_index: usize,
        unit_id: u32,
        destination: Position,
        embark_carrier_id: Option<u32>,
    ) {
        let unit = self.players[player_index]
            .get_unit_mut(unit_id)
            .expect("the player should have all units to move");
        unit.position = destination;
        unit.carrier_id = embark_carrier_id;

        if let Some(terrain) = terrain_movement_restriction(&self.map, destination, unit) {
            unit.movement_restrictions.push(terrain);
        }

        for id in carried_units(unit_id, &self.players[player_index]) {
            self.players[player_index]
                .get_unit_mut(id)
                .expect("the player should have all units to move")
                .position = destination;
        }
    }

    pub(crate) fn undo_move_units(
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

        for unit_id in units {
            let unit = self.players[player_index]
                .get_unit_mut(unit_id)
                .expect("the player should have all units to move");
            unit.position = starting_position;

            if let Some(terrain) = terrain_movement_restriction(&self.map, destination, unit) {
                unit.movement_restrictions
                    .iter()
                    .position(|r| r == &terrain)
                    .map(|i| unit.movement_restrictions.remove(i));
            }

            if !self.map.is_water(starting_position) {
                unit.carrier_id = None;
            }
            for id in &carried_units(unit_id, &self.players[player_index]) {
                self.players[player_index]
                    .get_unit_mut(*id)
                    .expect("the player should have all units to move")
                    .position = starting_position;
            }
        }
    }

    ///
    /// # Panics
    ///
    /// Panics if the player does not have the unit
    pub fn kill_unit(&mut self, unit_id: u32, player_index: usize, killer: usize) {
        let unit = self.players[player_index].remove_unit(unit_id);
        if matches!(unit.unit_type, UnitType::Leader) {
            let leader = self.players[player_index]
                .active_leader
                .take()
                .expect("A player should have an active leader when having a leader unit");
            Player::with_leader(&leader, self, player_index, |game, leader| {
                (leader.listeners.deinitializer)(game, player_index);
            });
            self.players[killer].captured_leaders.push(leader);
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
    state_change_event_state: Vec<CustomPhaseEventState>,
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
    #[serde(skip_serializing_if = "Rng::is_zero")]
    rng: Rng,
    dice_roll_log: Vec<u8>,
    dropped_players: Vec<usize>,
    wonders_left: Vec<String>,
    wonder_amount_left: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    incidents_left: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CulturalInfluenceResolution {
    pub roll_boost_cost: ResourcePile,
    pub target_player_index: usize,
    pub target_city_position: Position,
    pub city_piece: Building,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CurrentMove {
    #[default]
    None,
    Embark {
        source: Position,
        destination: Position,
    },
    Fleet {
        units: Vec<u32>,
    },
}

impl CurrentMove {
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, CurrentMove::None)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct MoveState {
    pub movement_actions_left: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub moved_units: Vec<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "CurrentMove::is_none")]
    pub current_move: CurrentMove,
}

impl Default for MoveState {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveState {
    #[must_use]
    pub fn new() -> Self {
        MoveState {
            movement_actions_left: MOVEMENT_ACTIONS,
            moved_units: Vec::new(),
            current_move: CurrentMove::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ExploreResolutionState {
    #[serde(flatten)]
    pub move_state: MoveState,
    pub block: UnexploredBlock,
    pub units: Vec<u32>,
    pub start: Position,
    pub destination: Position,
    pub ship_can_teleport: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum GameState {
    Playing,
    StatusPhase(StatusPhaseState),
    Movement(MoveState),
    CulturalInfluenceResolution(CulturalInfluenceResolution),
    Combat(Combat),
    ExploreResolution(ExploreResolutionState),
    Finished,
}

impl GameState {
    #[must_use]
    pub fn is_playing(&self) -> bool {
        matches!(self, Playing)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionLogItem {
    pub action: Action,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub undo: Vec<UndoContext>,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DisembarkUndoContext {
    unit_id: u32,
    carrier_id: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GainUnitContext {
    unit_type: UnitType,
    position: Position,
    player: usize,
}

impl GainUnitContext {
    #[must_use]
    pub fn new(unit_type: UnitType, position: Position, player: usize) -> Self {
        Self {
            unit_type,
            position,
            player,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GainCityContext {
    position: Position,
    player: usize,
}

impl GainCityContext {
    #[must_use]
    pub fn new(position: Position, player: usize) -> Self {
        Self { position, player }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandContext {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub info: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub gained_resources: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gained_units: Vec<GainUnitContext>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gained_cities: Vec<GainCityContext>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barbarian_update: Option<BarbariansEventState>,
}

impl CommandContext {
    #[must_use]
    pub fn new(info: HashMap<String, String>) -> Self {
        Self {
            info,
            gained_resources: ResourcePile::empty(),
            gained_units: Vec::new(),
            gained_cities: Vec::new(),
            barbarian_update: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandUndoInfo {
    pub player: usize,
    pub info: HashMap<String, String>,
}

impl CommandUndoInfo {
    #[must_use]
    pub fn new(player: &Player) -> Self {
        Self {
            info: player.event_info.clone(),
            player: player.index,
        }
    }

    pub fn apply(&self, game: &mut Game, mut undo: CommandContext) {
        let player = &mut game.players[self.player];
        for (k, v) in undo.info.clone() {
            player.event_info.insert(k, v);
        }

        if undo.info != self.info
            || !undo.gained_resources.is_empty()
            || !undo.gained_units.is_empty()
            || !undo.gained_cities.is_empty()
        {
            undo.info.clone_from(&self.info);
            game.push_undo_context(UndoContext::Command(undo));
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum UndoContext {
    FoundCity {
        settler: UnitData,
    },
    Recruit {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        #[serde(default)]
        replaced_units: Vec<UnitData>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        replaced_leader: Option<String>,
    },
    Movement {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        starting_position: Option<Position>,
        #[serde(flatten)]
        move_state: MoveState,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        disembarked_units: Vec<DisembarkUndoContext>,
    },
    ExploreResolution(ExploreResolutionState),
    WastedResources {
        resources: ResourcePile,
        player_index: usize,
    },
    IncreaseHappiness {
        angry_activations: Vec<Position>,
    },
    InfluenceCultureResolution {
        roll_boost_cost: ResourcePile,
    },
    CustomPhaseEvent(CustomPhaseEventState),
    Command(CommandContext),
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

    use super::{Game, GameState::Playing};

    use crate::payment::PaymentOptions;
    use crate::utils::tests::FloatEq;
    use crate::{
        city::{City, MoodState::*},
        city_pieces::Building::*,
        content::civilizations,
        map::Map,
        player::Player,
        position::Position,
        utils::Rng,
        wonder::Wonder,
    };

    #[must_use]
    pub fn test_game() -> Game {
        Game {
            state: Playing,
            custom_phase_state: Vec::new(),
            players: Vec::new(),
            map: Map::new(HashMap::new()),
            starting_player_index: 0,
            current_player_index: 0,
            action_log: Vec::new(),
            action_log_index: 0,
            log: [
                String::from("The game has started"),
                String::from("Age 1 has started"),
                String::from("Round 1/3"),
            ]
            .iter()
            .map(|s| vec![s.to_string()])
            .collect(),
            undo_limit: 0,
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            rng: Rng::from_seed(1_234_567_890),
            dice_roll_outcomes: Vec::new(),
            dice_roll_log: Vec::new(),
            dropped_players: Vec::new(),
            wonders_left: Vec::new(),
            wonder_amount_left: 0,
            incidents_left: Vec::new(),
            undo_context_stack: Vec::new(),
        }
    }

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", "test", PaymentOptions::free(), vec![]).build();
        let mut game = test_game();
        game.players.push(old);
        game.players.push(new);
        let old = 0;
        let new = 1;

        let position = Position::new(0, 0);
        game.players[old].cities.push(City::new(old, position));
        game.build_wonder(wonder, position, old);
        game.players[old].construct(Academy, position, None);
        game.players[old].construct(Obelisk, position, None);

        game.players[old].victory_points(&game).assert_eq(8.0);

        game.conquer_city(position, new, old);

        let c = game.players[new]
            .get_city_mut(position)
            .expect("player new should the city");
        assert_eq!(1, c.player_index);
        assert_eq!(Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        old.victory_points(&game).assert_eq(4.0);
        new.victory_points(&game).assert_eq(5.0);
        assert_eq!(0, old.wonders_owned());
        assert_eq!(1, new.wonders_owned());
        assert_eq!(1, old.owned_buildings(&game));
        assert_eq!(1, new.owned_buildings(&game));
    }
}
