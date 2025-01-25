use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem;

use GameState::*;

use crate::combat::{self, Combat, CombatDieRoll, CombatPhase, COMBAT_DIE_SIDES};
use crate::consts::MOVEMENT_ACTIONS;
use crate::content::custom_phase_actions::CustomPhaseState;
use crate::explore::{explore_resolution, move_to_unexplored_tile, undo_explore_resolution};
use crate::map::UnexploredBlock;
use crate::movement::{has_movable_units, terrain_movement_restriction};
use crate::resource::check_for_waste;
use crate::unit::{carried_units, get_current_move, MovementRestriction};
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
    pub players: Vec<Player>,
    pub map: Map,
    pub starting_player_index: usize,
    pub current_player_index: usize,
    pub action_log: Vec<Action>,
    pub action_log_index: usize,
    pub log: Vec<String>,
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
            let civilization = rng.range(0, civilizations.len());
            players.push(Player::new(civilizations.remove(civilization), i));
        }

        let starting_player = rng.range(0, players.len());

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
            actions_left: 3,
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

    pub(crate) fn add_action_log_item(&mut self, item: Action) {
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
        if matches!(action, Action::Redo) {
            assert!(self.can_redo(), "no action can be redone");
            self.redo(player_index);
            return;
        }
        self.log.push(log::format_action_log_item(&action, self));
        self.add_action_log_item(action.clone());
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
            Combat(c) => {
                let action = action.combat().expect("action should be a combat action");
                combat::execute_combat_action(self, action, c);
            }
            PlaceSettler(p) => self.place_settler(action, player_index, &p),
            ExploreResolution(r) => {
                let rotation = action
                    .explore_resolution()
                    .expect("action should be an explore resolution action");
                explore_resolution(self, &r, rotation);
            }
            CustomPhase(_) => {
                let action = action
                    .custom_phase()
                    .expect("action should be a custom phase action");
                action.execute(self, player_index);
            }
            Finished => panic!("actions can't be executed when the game is finished"),
        }
        check_for_waste(self, player_index);
    }

    fn undo(&mut self, player_index: usize) {
        match &self.action_log[self.action_log_index - 1] {
            Action::Playing(action) => action.clone().undo(self, player_index),
            Action::StatusPhase(_) => panic!("status phase actions can't be undone"),
            Action::Movement(action) => {
                self.undo_movement_action(action.clone(), player_index);
            }
            Action::CulturalInfluenceResolution(action) => {
                self.undo_cultural_influence_resolution_action(*action);
            }
            // todo: can remove casualties be undone?
            Action::Combat(_action) => unimplemented!("retreat can't yet be undone"),
            Action::PlaceSettler(_action) => panic!("placing a settler can't be undone"),
            Action::ExploreResolution(_rotation) => {
                undo_explore_resolution(self, player_index);
            }
            Action::CustomPhase(action) => action.clone().undo(self, player_index),
            Action::Undo => panic!("undo action can't be undone"),
            Action::Redo => panic!("redo action can't be undone"),
        }
        self.action_log_index -= 1;
        self.log.remove(self.log.len() - 1);
        if let Some(UndoContext::WastedResources { resources }) = self.undo_context_stack.last() {
            self.players[player_index].gain_resources(resources.clone());
            self.undo_context_stack.pop();
        }
    }

    fn redo(&mut self, player_index: usize) {
        let action_log_item = &self.action_log[self.action_log_index];
        self.log
            .push(log::format_action_log_item(&action_log_item.clone(), self));
        match action_log_item {
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
                    c.roll_boost_cost,
                    c.target_player_index,
                    c.target_city_position,
                    c.city_piece,
                    player_index,
                );
            }
            Action::Combat(_) => unimplemented!("retreat can't yet be redone"),
            Action::PlaceSettler(_) => panic!("place settler actions can't be redone"),
            Action::ExploreResolution(rotation) => {
                let ExploreResolution(r) = &self.state else {
                    panic!("explore resolution actions can only be redone if the game is in a explore resolution state");
                };
                explore_resolution(self, &r.clone(), *rotation);
            }
            Action::CustomPhase(action) => {
                action.clone().execute(self, player_index);
            }
            Action::Undo => panic!("undo action can't be redone"),
            Action::Redo => panic!("redo action can't be redone"),
        }
        self.action_log_index += 1;
        check_for_waste(self, player_index);
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
        let mut cost = None;
        let (starting_position, disembarked_units) = match action {
            Move {
                units,
                destination,
                embark_carrier_id,
            } => {
                if let Playing = self.state {
                    assert_ne!(self.actions_left, 0, "Illegal action");
                    self.actions_left -= 1;
                }
                let player = &self.players[player_index];
                let starting_position = player
                    .get_unit(*units.first().expect(
                        "instead of providing no units to move a stop movement actions should be done",
                    ))
                    .expect("the player should have all units to move")
                    .position;
                let disembarked_units = units
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
                    &units,
                    starting_position,
                    embark_carrier_id,
                ) {
                    Ok(destinations) => {
                        let c = &destinations
                            .iter()
                            .find(|route| route.destination == destination)
                            .expect("destination should be a valid destination")
                            .cost;
                        if !c.is_empty() {
                            self.players[player_index].loose_resources(c.clone());
                            cost = Some(c.clone());
                        }
                    }
                    Err(e) => {
                        panic!("cannot move units to destination: {e}");
                    }
                }

                move_state.moved_units.extend(units.iter());
                move_state.moved_units = move_state.moved_units.iter().unique().copied().collect();
                let current_move = get_current_move(
                    self,
                    &units,
                    starting_position,
                    destination,
                    embark_carrier_id,
                );
                if matches!(current_move, CurrentMove::None)
                    || move_state.current_move != current_move
                {
                    move_state.movement_actions_left -= 1;
                    move_state.current_move = current_move;
                }

                let dest_terrain = self
                    .map
                    .get(destination)
                    .expect("destination should be a valid tile");
                if dest_terrain == &Unexplored {
                    if move_to_unexplored_tile(
                        self,
                        player_index,
                        &units,
                        starting_position,
                        destination,
                        &move_state,
                    ) {
                        self.back_to_move(&move_state, true);
                    }
                    return;
                }

                let enemy = self.enemy_player(player_index, destination);
                if let Some(defender) = enemy {
                    if self.move_to_defended_tile(
                        player_index,
                        &mut move_state,
                        &units,
                        destination,
                        starting_position,
                        defender,
                    ) {
                        return;
                    }
                } else {
                    self.move_units(player_index, &units, destination, embark_carrier_id);
                }

                self.back_to_move(&move_state, !starting_position.is_neighbor(destination));

                if let Some(enemy) = enemy {
                    self.capture_position(enemy, destination, player_index);
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
            cost,
        });
    }

    pub(crate) fn push_undo_context(&mut self, context: UndoContext) {
        self.undo_context_stack.push(context);
    }

    fn move_to_defended_tile(
        &mut self,
        player_index: usize,
        move_state: &mut MoveState,
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
            // move to destination to apply movement restrictions, etc.
            self.move_unit(player_index, *unit_id, destination, None);
        }
        assert!(military, "Need military units to attack");
        self.back_to_move(move_state, true);

        if has_defending_units || has_fortress {
            combat::initiate_combat(
                self,
                defender,
                destination,
                player_index,
                starting_position,
                units.clone(),
                true,
                None,
            );
            if !matches!(self.state, Movement(_)) {
                for unit_id in units {
                    // but the unit is still in the starting position until the attack is resolved
                    // mostly to keep the logic clean
                    self.players[player_index]
                        .get_unit_mut(*unit_id)
                        .expect("the player should have all units to move")
                        .position = starting_position;
                }
                return true;
            }
        }
        false
    }

    fn undo_movement_action(&mut self, action: MovementAction, player_index: usize) {
        let Some(UndoContext::Movement {
            starting_position,
            move_state,
            disembarked_units,
            cost,
        }) = self.undo_context_stack.pop()
        else {
            panic!("when undoing a movement action, the game should have stored movement context")
        };
        if let Move {
            units,
            destination: _,
            embark_carrier_id: _,
        } = action
        {
            self.undo_move_units(
                player_index,
                units,
                starting_position.expect(
                    "undo context should contain the starting position if units where moved",
                ),
            );
            if let Some(cost) = cost {
                self.players[player_index].gain_resources(cost);
            }
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
            target_city_position,
            city_piece,
        );
    }

    fn undo_cultural_influence_resolution_action(&mut self, action: bool) {
        let cultural_influence_attempt_action = self.action_log[self.action_log_index - 2].playing_ref().expect("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        let PlayingAction::InfluenceCultureAttempt(c) = cultural_influence_attempt_action else {
            panic!("any log item previous to a cultural influence resolution action log item should a cultural influence attempt action log item");
        };
        let roll =
            self.dice_roll_log.last().expect(
                "there should be a dice roll before a cultural influence resolution action",
            ) / 2
                + 1;
        let roll_boost_cost = 5 - roll as u32;
        let city_piece = c.city_piece;
        let target_player_index = c.target_player_index;
        let target_city_position = c.target_city_position;
        self.state = GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
            roll_boost_cost,
            target_player_index,
            target_city_position,
            city_piece,
        });
        if !action {
            return;
        }
        self.players[self.current_player_index]
            .gain_resources(ResourcePile::culture_tokens(roll_boost_cost));
        self.undo_influence_culture(target_player_index, target_city_position, city_piece);
    }

    fn place_settler(&mut self, action: Action, player_index: usize, p: &PlaceSettlerState) {
        let action = action
            .place_settler()
            .expect("action should be place_settler action");
        let player = &mut self.players[player_index];
        assert!(player.get_city(action).is_some(), "Illegal action");
        player.add_unit(action, UnitType::Settler);
        self.back_to_move(&p.move_state, true);
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
            PlaceSettler(p) => p.player_index,
            _ => self.current_player_index,
        }
    }

    pub fn next_turn(&mut self) {
        self.actions_left = 3;
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
            .max_by(|(_, player), (_, other)| player.compare_score(other, self))
            .expect("there should be at least one player in the game")
            .0;
        let winner_name = self.players[winner_player_index].get_name();
        self.add_info_log_item(format!("The game has ended\n{winner_name} has won"));
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
    pub fn get_available_custom_actions(&self, player_index: usize) -> Vec<CustomActionType> {
        let custom_actions = &self.players[self.current_player_index].custom_actions;
        custom_actions
            .iter()
            .filter(|&action| {
                !self
                    .get_player(player_index)
                    .played_once_per_turn_actions
                    .contains(action)
                    && action.is_available(self, player_index)
            })
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

    fn set_active_leader(&mut self, leader_name: String, player_index: usize) {
        self.players[player_index]
            .available_leaders
            .retain(|name| name != &leader_name);
        Player::with_leader(&leader_name, self, player_index, |game, leader| {
            (leader.player_initializer)(game, player_index);
            (leader.player_one_time_initializer)(game, player_index);
        });
        self.players[player_index].active_leader = Some(leader_name);
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
        let advance = advances::get_advance_by_name(advance);
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
        if let Some(advance_bonus) = &advance.bonus {
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
        let advance = advances::get_advance_by_name(advance);
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
        if let Some(advance_bonus) = &advance.bonus {
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
        leader_name: Option<&String>,
        replaced_units: Vec<u32>,
    ) {
        let mut replaced_leader = None;
        if let Some(leader_name) = leader_name {
            if let Some(previous_leader) = self.players[player_index].active_leader.take() {
                Player::with_leader(
                    &previous_leader,
                    self,
                    player_index,
                    |game, previous_leader| {
                        (previous_leader.player_deinitializer)(game, player_index);
                    },
                );
                replaced_leader = Some(previous_leader);
            }
            self.set_active_leader(leader_name.clone(), player_index);
        }
        let mut replaced_units_undo_context = Vec::new();
        for unit in replaced_units {
            let player = &mut self.players[player_index];
            let unit = player
                .remove_unit(unit)
                .expect("the player should have the replaced units");
            replaced_units_undo_context.push(unit);
        }
        self.push_undo_context(UndoContext::Recruit {
            replaced_units: replaced_units_undo_context,
            replaced_leader,
        });
        let player = &mut self.players[player_index];
        let mut ships = Vec::new();
        player.units.reserve_exact(units.len());
        for unit_type in units {
            let city = player
                .get_city(city_position)
                .expect("player should have a city at the recruitment position");
            let position = match &unit_type {
                UnitType::Ship => {
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

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn undo_recruit(
        &mut self,
        player_index: usize,
        units: &[UnitType],
        city_position: Position,
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
                    (current_leader.player_deinitializer)(game, player_index);
                    (current_leader.player_undo_deinitializer)(game, player_index);
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
                player.units.push(unit);
            }
            if let Some(replaced_leader) = replaced_leader {
                player.active_leader = Some(replaced_leader.clone());
                Player::with_leader(
                    &replaced_leader,
                    self,
                    player_index,
                    |game, replaced_leader| {
                        (replaced_leader.player_initializer)(game, player_index);
                        (replaced_leader.player_one_time_initializer)(game, player_index);
                    },
                );
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
        let advance = advances::get_advance_by_name(advance);
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
        self.players[new_player_index]
            .gain_resources(ResourcePile::gold(city.mood_modified_size() as u32));
        let take_over = self.players[new_player_index].is_city_available();

        if take_over {
            for wonder in &city.pieces.wonders {
                (wonder.player_deinitializer)(self, old_player_index);
                (wonder.player_initializer)(self, new_player_index);
            }
            city.player_index = new_player_index;
            city.mood_state = Angry;

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
                city.pieces.set_building(building, new_player_index);
                if !(self.players[new_player_index].is_building_available(building, self)) {
                    city.pieces.remove_building(building);
                    self.players[new_player_index].gain_resources(ResourcePile::gold(1));
                }
            }
        }
        if take_over {
            self.players[new_player_index].cities.push(city);
        } else {
            self.players[new_player_index].gain_resources(ResourcePile::gold(city.size() as u32));
            city.raze(self, old_player_index);
        }
        let old_player = &mut self.players[old_player_index];
        if old_player.available_units().settlers > 0 && !old_player.cities.is_empty() {
            let state = mem::replace(&mut self.state, Playing);
            let Movement(m) = state else {
                panic!("conquering a city should only happen in a movement action")
            };
            self.state = PlaceSettler(PlaceSettlerState {
                player_index: old_player_index,
                move_state: m,
            });
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
                " and captured {} settlers of {}",
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
        self.players[player_index].take_events(|events, player| {
            events
                .on_construct_wonder
                .trigger(player, &city_position, &wonder);
        });
        let wonder = wonder;
        (wonder.player_initializer)(self, player_index);
        (wonder.player_one_time_initializer)(self, player_index);
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
        self.players[player_index].take_events(|events, player| {
            events
                .on_undo_construct_wonder
                .trigger(player, &city_position, &wonder);
        });
        (wonder.player_deinitializer)(self, player_index);
        (wonder.player_undo_deinitializer)(self, player_index);
        wonder
    }

    #[must_use]
    pub fn influence_culture_boost_cost(
        &self,
        player_index: usize,
        starting_city_position: Position,
        target_player_index: usize,
        target_city_position: Position,
        city_piece: Building,
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
        if matches!(city_piece, Building::Obelisk)
            || starting_city.player_index != player_index
            || !player.can_afford_resources(&range_boost_cost)
            || (starting_city.influenced() && !self_influence)
            || self.successful_cultural_influence
            || !player.is_building_available(city_piece, self)
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
        building: Building,
    ) {
        self.players[influenced_player_index]
            .get_city_mut(city_position)
            .expect("influenced player should have influenced city")
            .pieces
            .set_building(building, influencer_index);
        self.successful_cultural_influence = true;
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
        destination: Position,
        embark_carrier_id: Option<u32>,
    ) {
        for unit_id in units {
            self.move_unit(player_index, *unit_id, destination, embark_carrier_id);
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

        for id in carried_units(self, player_index, unit_id) {
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
            for id in &carried_units(self, player_index, unit_id) {
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
        if let Some(unit) = self.players[player_index].remove_unit(unit_id) {
            if matches!(unit.unit_type, UnitType::Leader) {
                let leader = self.players[player_index]
                    .active_leader
                    .take()
                    .expect("A player should have an active leader when having a leader unit");
                Player::with_leader(&leader, self, player_index, |game, leader| {
                    (leader.player_deinitializer)(game, player_index);
                });
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

#[derive(Serialize, Deserialize)]
pub struct GameData {
    state: GameState,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    action_log: Vec<Action>,
    action_log_index: usize,
    log: Vec<String>,
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
    undo_context_stack: Vec<UndoContext>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CulturalInfluenceResolution {
    pub roll_boost_cost: u32,
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
pub struct PlaceSettlerState {
    pub player_index: usize,
    #[serde(flatten)]
    pub move_state: MoveState,
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
    PlaceSettler(PlaceSettlerState),
    ExploreResolution(ExploreResolutionState),
    CustomPhase(CustomPhaseState),
    Finished,
}

impl GameState {
    #[must_use]
    pub fn settler_placer(&self) -> Option<usize> {
        match self {
            PlaceSettler(p) => Some(p.player_index),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_playing(&self) -> bool {
        matches!(self, Playing)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DisembarkUndoContext {
    unit_id: u32,
    carrier_id: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum UndoContext {
    FoundCity {
        settler: Unit,
    },
    Recruit {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        #[serde(default)]
        replaced_units: Vec<Unit>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        cost: Option<ResourcePile>,
    },
    ExploreResolution(ExploreResolutionState),
    WastedResources {
        resources: ResourcePile,
    },
    IncreaseHappiness {
        angry_activations: Vec<Position>
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

    use super::{Game, GameState::Playing};
    use crate::payment::PaymentModel;
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
            undo_context_stack: Vec::new(),
        }
    }

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", PaymentModel::free(), vec![]).build();
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
