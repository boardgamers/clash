use crate::advance::Advance;
use crate::game::CurrentMove;
use crate::game::GameState::Movement;
use crate::unit::{carried_units, get_current_move, land_movement};
use crate::{
    city::{City, CityData},
    city_pieces::Building::{self, *},
    civilization::Civilization,
    consts::{
        ADVANCE_COST, ADVANCE_VICTORY_POINTS, ARMY_MOVEMENT_REQUIRED_ADVANCE,
        BUILDING_VICTORY_POINTS, CAPTURED_LEADER_VICTORY_POINTS, CITY_LIMIT, CITY_PIECE_LIMIT,
        CONSTRUCT_COST, OBJECTIVE_VICTORY_POINTS, STACK_LIMIT, UNIT_LIMIT, WONDER_VICTORY_POINTS,
    },
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    game::Game,
    leader::Leader,
    map::Terrain::{self, *},
    player_events::PlayerEvents,
    position::Position,
    resource_pile::{AdvancePaymentOptions, ResourcePile},
    unit::{
        Unit,
        UnitType::{self, *},
        Units,
    },
    utils,
    wonder::Wonder,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering::{self, *},
    collections::{HashMap, HashSet},
    mem,
};

pub struct Player {
    name: Option<String>,
    pub index: usize,
    pub resources: ResourcePile,
    pub resource_limit: ResourcePile,
    pub events: Option<PlayerEvents>,
    pub cities: Vec<City>,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub advances: Vec<String>,
    pub unlocked_special_advances: Vec<String>,
    pub wonders_build: Vec<String>,
    pub game_event_tokens: u8,
    pub completed_objectives: Vec<String>,
    pub captured_leaders: Vec<String>,
    pub event_victory_points: f32,
    pub custom_actions: HashSet<CustomActionType>,
    pub wonder_cards: Vec<Wonder>,
    pub collect_options: HashMap<Terrain, Vec<ResourcePile>>,
    pub next_unit_id: u32,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        let data = self.cloned_data();
        Self::from_data(data)
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.index == other.index
            && self.resources == other.resources
            && self.resource_limit == other.resource_limit
            && self
                .cities
                .iter()
                .enumerate()
                .all(|(i, city)| city.position == other.cities[i].position)
            && self.units == other.units
            && self.civilization.name == other.civilization.name
            && self.active_leader.as_ref().map(|leader| &leader.name)
                == other.active_leader.as_ref().map(|leader| &leader.name)
            && self
                .available_leaders
                .iter()
                .enumerate()
                .all(|(i, leader)| leader.name == other.available_leaders[i].name)
            && self.advances == other.advances
            && self.unlocked_special_advances == other.unlocked_special_advances
            && self.wonders_build == other.wonders_build
            && self.game_event_tokens == other.game_event_tokens
            && self.completed_objectives == other.completed_objectives
            && self.captured_leaders == other.captured_leaders
            && self.event_victory_points == other.event_victory_points
            && self
                .wonder_cards
                .iter()
                .enumerate()
                .all(|(i, wonder)| wonder.name == other.wonder_cards[i].name)
            && self.collect_options == other.collect_options
    }
}

impl Player {
    ///
    ///
    /// # Panics
    ///
    /// Panics if elements like wonders or advances don't exist
    pub fn initialize_player(data: PlayerData, game: &mut Game) {
        let player = Self::from_data(data);
        let player_index = player.index;
        game.players.push(player);
        let advances = mem::take(&mut game.players[player_index].advances);
        for advance in &advances {
            let advance = advances::get_advance_by_name(advance);
            (advance.player_initializer)(game, player_index);
            for i in 0..game.players[player_index]
                .civilization
                .special_advances
                .len()
            {
                if game.players[player_index].civilization.special_advances[i].required_advance
                    == advance.name
                {
                    let special_advance = game.players[player_index]
                        .civilization
                        .special_advances
                        .remove(i);
                    (special_advance.player_initializer)(game, player_index);
                    game.players[player_index]
                        .civilization
                        .special_advances
                        .insert(i, special_advance);
                    break;
                }
            }
        }
        if let Some(leader) = game.players[player_index].active_leader.take() {
            (leader.player_initializer)(game, player_index);
            game.players[player_index].active_leader = Some(leader);
        }
        let mut cities = mem::take(&mut game.players[player_index].cities);
        for city in &mut cities {
            for wonder in &city.pieces.wonders {
                (wonder.player_initializer)(game, player_index);
            }
        }
        game.players[player_index].cities = cities;
        game.players[player_index].advances = advances;
    }

    fn from_data(data: PlayerData) -> Player {
        let player = Self {
            name: data.name,
            index: data.id,
            resources: data.resources,
            resource_limit: data.resource_limit,
            events: Some(PlayerEvents::default()),
            cities: data.cities.into_iter().map(City::from_data).collect(),
            units: data.units,
            civilization: civilizations::get_civilization_by_name(&data.civilization)
                .expect("player data should have a valid civilization"),
            active_leader: data.active_leader.map(|leader| {
                civilizations::get_leader_by_name(&leader, &data.civilization)
                    .expect("player data should contain a valid leader")
            }),
            available_leaders: data
                .available_leaders
                .into_iter()
                .map(|leader| {
                    civilizations::get_leader_by_name(&leader, &data.civilization)
                        .expect("player data should contain valid leaders")
                })
                .collect(),
            advances: data.advances,
            unlocked_special_advances: data.unlocked_special_advance,
            wonders_build: data.wonders_build,
            game_event_tokens: data.game_event_tokens,
            completed_objectives: data.completed_objectives,
            captured_leaders: data.captured_leaders,
            event_victory_points: data.event_victory_points,
            custom_actions: HashSet::new(),
            wonder_cards: data
                .wonder_cards
                .iter()
                .map(|wonder| {
                    wonders::get_wonder_by_name(wonder)
                        .expect("player data should have valid wonder cards")
                })
                .collect(),
            collect_options: data.collect_options.into_iter().collect(),
            next_unit_id: data.next_unit_id,
            played_once_per_turn_actions: data.played_once_per_turn_actions,
        };
        player
    }

    #[must_use]
    pub fn data(self) -> PlayerData {
        PlayerData {
            name: self.name,
            id: self.index,
            resources: self.resources,
            resource_limit: self.resource_limit,
            cities: self.cities.into_iter().map(City::data).collect(),
            units: self
                .units
                .into_iter()
                .sorted_by_key(|unit| unit.id)
                .collect(),
            civilization: self.civilization.name,
            active_leader: self.active_leader.map(|leader| leader.name),
            available_leaders: self
                .available_leaders
                .into_iter()
                .map(|leader| leader.name)
                .collect(),
            advances: self.advances.into_iter().sorted().collect(),
            unlocked_special_advance: self.unlocked_special_advances,
            wonders_build: self.wonders_build,
            game_event_tokens: self.game_event_tokens,
            completed_objectives: self.completed_objectives,
            captured_leaders: self.captured_leaders,
            event_victory_points: self.event_victory_points,
            wonder_cards: self
                .wonder_cards
                .into_iter()
                .map(|wonder| wonder.name)
                .collect(),
            collect_options: self
                .collect_options
                .into_iter()
                .sorted_by_key(|(terrain, _)| terrain.clone())
                .collect(),
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
        }
    }

    pub fn cloned_data(&self) -> PlayerData {
        PlayerData {
            name: self.name.clone(),
            id: self.index,
            resources: self.resources.clone(),
            resource_limit: self.resource_limit.clone(),
            cities: self.cities.iter().map(City::cloned_data).collect(),
            units: self
                .units
                .iter()
                .cloned()
                .sorted_by_key(|unit| unit.id)
                .collect(),
            civilization: self.civilization.name.clone(),
            active_leader: self
                .active_leader
                .as_ref()
                .map(|leader| leader.name.clone()),
            available_leaders: self
                .available_leaders
                .iter()
                .map(|leader| leader.name.clone())
                .collect(),
            advances: self.advances.iter().cloned().sorted().collect(),
            unlocked_special_advance: self.unlocked_special_advances.clone(),
            wonders_build: self.wonders_build.clone(),
            game_event_tokens: self.game_event_tokens,
            completed_objectives: self.completed_objectives.clone(),
            captured_leaders: self.captured_leaders.clone(),
            event_victory_points: self.event_victory_points,
            wonder_cards: self
                .wonder_cards
                .iter()
                .map(|wonder| wonder.name.clone())
                .collect(),
            collect_options: self
                .collect_options
                .iter()
                .map(|(terrain, options)| (terrain.clone(), options.clone()))
                .sorted_by_key(|(terrain, _)| terrain.clone())
                .collect(),
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions.clone(),
        }
    }

    #[must_use]
    pub fn new(civilization: Civilization, index: usize) -> Self {
        Self {
            name: None,
            index,
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(2, 7, 7, 7, 7, 0, 0),
            events: Some(PlayerEvents::new()),
            cities: Vec::new(),
            units: Vec::new(),
            civilization,
            active_leader: None,
            available_leaders: Vec::new(),
            advances: vec![String::from("Farming"), String::from("Mining")],
            unlocked_special_advances: Vec::new(),
            game_event_tokens: 3,
            completed_objectives: Vec::new(),
            captured_leaders: Vec::new(),
            event_victory_points: 0.0,
            custom_actions: HashSet::new(),
            wonder_cards: Vec::new(),
            wonders_build: Vec::new(),
            collect_options: HashMap::from([
                (Mountain, vec![ResourcePile::ore(1)]),
                (Fertile, vec![ResourcePile::food(1)]),
                (Forest, vec![ResourcePile::wood(1)]),
            ]),
            next_unit_id: 0,
            played_once_per_turn_actions: Vec::new(),
        }
    }

    pub fn end_turn(&mut self) {
        for city in &mut self.cities {
            city.deactivate();
        }
        for unit in &mut self.units {
            unit.reset_movement_restriction();
        }
        self.played_once_per_turn_actions.clear();
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    #[must_use]
    pub fn get_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or(format!("Player{}", self.index + 1))
    }

    /// Returns the government of this [`Player`].
    ///
    /// # Panics
    ///
    /// Panics if the player has advances which don't exist
    #[must_use]
    pub fn government(&self) -> Option<String> {
        self.advances
            .iter()
            .find_map(|advance| advances::get_advance_by_name(advance).government)
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.resources += resources;
        self.resources.apply_resource_limit(&self.resource_limit);
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if player cannot afford the resources
    pub fn loose_resources(&mut self, resources: ResourcePile) {
        assert!(
            self.resources.can_afford(&resources),
            "player should be able to afford the resources"
        );
        self.resources -= resources;
    }

    #[must_use]
    pub fn can_advance_in_change_government(&self, advance: &Advance) -> bool {
        if self.has_advance(&advance.name) {
            return false;
        }
        if let Some(required_advance) = &advance.required {
            if !self.has_advance(required_advance) {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn can_advance_free(&self, advance: &Advance) -> bool {
        for contradicting_advance in &advance.contradicting {
            if self.has_advance(contradicting_advance) {
                return false;
            }
        }
        self.can_advance_in_change_government(advance)
    }

    #[must_use]
    pub fn can_advance(&self, advance: &Advance) -> bool {
        if self.resources.food + self.resources.ideas + (self.resources.gold as u32)
            < self.advance_cost(&advance.name)
        {
            return false;
        }
        self.can_advance_free(advance)
    }

    #[must_use]
    pub fn has_advance(&self, advance: &str) -> bool {
        self.advances.iter().any(|advances| advances == advance)
    }

    #[must_use]
    pub fn victory_points(&self, game: &Game) -> f32 {
        self.victory_points_parts(game).iter().sum()
    }

    #[must_use]
    pub fn victory_points_parts(&self, game: &Game) -> [f32; 6] {
        [
            (self.cities.len() + self.owned_buildings(game)) as f32 * BUILDING_VICTORY_POINTS,
            (self.advances.len() + self.unlocked_special_advances.len()) as f32
                * ADVANCE_VICTORY_POINTS,
            self.completed_objectives.len() as f32 * OBJECTIVE_VICTORY_POINTS,
            (self.wonders_owned() + self.wonders_build.len()) as f32 * WONDER_VICTORY_POINTS / 2.0,
            self.event_victory_points,
            self.captured_leaders.len() as f32 * CAPTURED_LEADER_VICTORY_POINTS,
        ]
    }

    #[must_use]
    pub fn owned_buildings(&self, game: &Game) -> usize {
        game.players
            .iter()
            .flat_map(|player| &player.cities)
            .map(|city| city.pieces.buildings(Some(self.index)).len())
            .sum()
    }

    #[must_use]
    pub fn wonders_owned(&self) -> usize {
        self.cities
            .iter()
            .map(|city| city.pieces.wonders.len())
            .sum::<usize>()
    }

    #[must_use]
    pub fn is_building_available(&self, building: Building, game: &Game) -> bool {
        game.players
            .iter()
            .flat_map(|player| &player.cities)
            .flat_map(|city| city.pieces.building_owners())
            .filter(|(b, owner)| b == &building && owner.is_some_and(|owner| owner == self.index))
            .count()
            < CITY_PIECE_LIMIT
    }

    #[must_use]
    pub fn is_city_available(&self) -> bool {
        self.cities.len() < CITY_LIMIT as usize
    }

    #[must_use]
    pub fn available_units(&self) -> Units {
        let mut units = UNIT_LIMIT.clone();
        for u in &self.units {
            units -= &u.unit_type;
        }
        units
    }

    pub fn remove_wonder(&mut self, wonder: &Wonder) {
        utils::remove_element(&mut self.wonders_build, &wonder.name);
    }

    #[must_use]
    pub fn game_event_tokens(&self) -> u8 {
        self.game_event_tokens
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = Vec::new();
        //todo strip information about other hand cards
    }

    #[must_use]
    pub(crate) fn compare_score(&self, other: &Self, game: &Game) -> Ordering {
        let parts = self.victory_points_parts(game);
        let other_parts = other.victory_points_parts(game);
        let sum = parts.iter().sum::<f32>();
        let other_sum = other_parts.iter().sum::<f32>();

        match sum.partial_cmp(&other_sum).expect("should be able to compare") {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }

        for (part, other_part) in parts.iter().zip(other_parts.iter()) {
            match part.partial_cmp(other_part).expect("should be able to compare") {
                Less => return Less,
                Equal => (),
                Greater => return Greater,
            }
        }
        Equal
    }

    #[must_use]
    pub fn construct_cost(&self, building: Building, city: &City) -> ResourcePile {
        let mut cost = CONSTRUCT_COST;
        self.get_events()
            .construct_cost
            .trigger(&mut cost, city, &building);
        cost
    }

    #[must_use]
    pub fn wonder_cost(&self, wonder: &Wonder, city: &City) -> ResourcePile {
        let mut cost = wonder.cost.clone();
        self.get_events()
            .wonder_cost
            .trigger(&mut cost, city, wonder);
        cost
    }

    #[must_use]
    pub fn advance_cost(&self, advance: &str) -> u32 {
        let mut cost = ADVANCE_COST;
        self.get_events()
            .advance_cost
            .trigger(&mut cost, &advance.to_string(), &());
        cost
    }

    #[must_use]
    pub fn get_advance_payment_options(&self, advance: &str) -> AdvancePaymentOptions {
        self.resources
            .get_advance_payment_options(self.advance_cost(advance))
    }

    #[must_use]
    pub fn get_city(&self, position: Position) -> Option<&City> {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)?;
        Some(&self.cities[position])
    }

    pub fn get_city_mut(&mut self, position: Position) -> Option<&mut City> {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)?;
        Some(&mut self.cities[position])
    }

    pub fn take_city(&mut self, position: Position) -> Option<City> {
        Some(
            self.cities.remove(
                self.cities
                    .iter()
                    .position(|city| city.position == position)?,
            ),
        )
    }

    #[must_use]
    pub fn can_raze_city(&self, city_position: Position) -> bool {
        self.cities.len() > 1
            && self
                .get_city(city_position)
                .is_some_and(|city| city.size() == 1)
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn construct(
        &mut self,
        building: Building,
        city_position: Position,
        port_position: Option<Position>,
    ) {
        self.take_events(|events, player| {
            events
                .on_construct
                .trigger(player, &city_position, &building);
        });
        let index = self.index;
        let city = self
            .get_city_mut(city_position)
            .expect("player should be have the this city");
        city.activate();
        city.pieces.set_building(building, index);
        if let Some(port_position) = port_position {
            city.port_position = Some(port_position);
        }
        if matches!(building, Academy) {
            self.gain_resources(ResourcePile::ideas(2));
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    pub fn undo_construct(&mut self, building: Building, city_position: Position) {
        self.take_events(|events, player| {
            events
                .on_undo_construct
                .trigger(player, &city_position, &building);
        });
        let city = self
            .get_city_mut(city_position)
            .expect("player should have city");
        city.undo_activate();
        city.pieces.remove_building(building);
        if matches!(building, Port) {
            city.port_position = None;
        }
        if matches!(building, Academy) {
            self.loose_resources(ResourcePile::ideas(2));
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    #[must_use]
    pub fn can_recruit(
        &self,
        units: &[UnitType],
        city_position: Position,
        leader_index: Option<usize>,
        replaced_units: &[u32],
    ) -> bool {
        if !self.can_recruit_without_replaced(units, city_position, leader_index) {
            return false;
        }
        let mut units_left = self.available_units();
        let mut required_units = Units::empty();
        for unit in units {
            if !units_left.has_unit(unit) {
                required_units += unit;
                continue;
            }
            units_left -= unit;
        }
        let replaced_units = replaced_units
            .iter()
            .map(|id| {
                self.get_unit(*id)
                    .expect("player should have units to be replaced")
                    .unit_type
                    .clone()
            })
            .collect();
        if required_units != replaced_units {
            return false;
        }
        true
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if city does not exist
    #[must_use]
    pub fn can_recruit_without_replaced(
        &self,
        units: &[UnitType],
        city_position: Position,
        leader_index: Option<usize>,
    ) -> bool {
        let city = self
            .get_city(city_position)
            .expect("player should have a city at the recruitment position");
        if !city.can_activate() {
            return false;
        }
        let cost = units.iter().map(UnitType::cost).sum();
        if !self.resources.can_afford(&cost) {
            return false;
        }
        if units.len() > city.mood_modified_size() {
            return false;
        }
        if units.iter().any(|unit| matches!(unit, Cavalry | Elephant))
            && city.pieces.market.is_none()
        {
            return false;
        }
        if units.iter().any(|unit| matches!(unit, Ship)) && city.pieces.port.is_none() {
            return false;
        }
        if self
            .get_units(city_position)
            .iter()
            .filter(|unit| unit.unit_type.is_army_unit())
            .count()
            + units.iter().filter(|unit| unit.is_army_unit()).count()
            > STACK_LIMIT
        {
            return false;
        }
        if units.iter().any(|unit| matches!(unit, UnitType::Leader))
            && (leader_index.is_none()
                || leader_index.is_none_or(|index| index >= self.available_leaders.len()))
        {
            return false;
        }
        if units
            .iter()
            .filter(|unit| matches!(unit, UnitType::Leader))
            .count()
            > 1
        {
            return false;
        }
        if leader_index.is_some_and(|leader_index| leader_index >= self.available_leaders.len()) {
            return false;
        }
        true
    }

    pub fn add_unit(&mut self, position: Position, unit_type: UnitType) {
        let unit = Unit::new(self.index, position, unit_type, self.next_unit_id);
        self.units.push(unit);
        self.next_unit_id += 1;
    }

    /// # Errors
    ///
    /// Will return `Err` if the unit cannot move to the destination.
    ///
    /// # Panics
    ///
    /// Panics if destination tile does not exist
    pub fn can_move_units(
        &self,
        game: &Game,
        units: &[u32],
        starting: Position,
        destination: Position,
        embark_carrier_id: Option<u32>,
    ) -> Result<(), String> {
        let (moved_units, movement_actions_left, current_move) = if let Movement(m) = &game.state {
            (&m.moved_units, m.movement_actions_left, &m.current_move)
        } else {
            (&vec![], 1, &CurrentMove::None)
        };

        if units.is_empty() {
            return Err("no units to move".to_string());
        }

        if !starting.is_neighbor(destination) {
            return Err("the destination should be adjacent to the starting position".to_string());
        }
        if matches!(current_move, CurrentMove::None)
            || current_move
                != &get_current_move(game, units, starting, destination, embark_carrier_id)
        {
            if movement_actions_left == 0 {
                return Err("no movement actions left".to_string());
            }

            if units.iter().any(|unit| moved_units.contains(unit)) {
                return Err("some units have already moved".to_string());
            }
        }

        if embark_carrier_id
            .is_some_and(|id| carried_units(game, self.index, id).len() + units.len() > 2)
        {
            return Err("carrier capacity exceeded".to_string());
        }

        let land_movement = land_movement(game, destination);
        let mut stack_size = 0;

        for unit_id in units {
            let unit = self
                .get_unit(*unit_id)
                .ok_or("the player should have all units to move")?;

            if unit.position != starting {
                return Err("the unit should be at the starting position".to_string());
            }
            if !unit.can_move() {
                return Err("the unit should be able to move".to_string());
            }
            if let Some(embark_carrier_id) = embark_carrier_id {
                if !unit.unit_type.is_land_based() {
                    return Err("the unit should be land based to embark".to_string());
                }
                let carrier = self
                    .get_unit(embark_carrier_id)
                    .ok_or("the player should have the carrier unit")?;
                if !carrier.unit_type.is_ship() {
                    return Err("the carrier should be a ship".to_string());
                }
                if carrier.position != destination {
                    return Err("the carrier should be at the destination position".to_string());
                }
            } else if unit.unit_type.is_land_based() != land_movement {
                return Err("the unit cannot move here".to_string());
            }
            if unit.unit_type.is_army_unit() && !self.has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE) {
                return Err("army movement advance missing".to_string());
            }
            if unit.unit_type.is_army_unit() && !unit.unit_type.is_settler() {
                stack_size += 1;
            }
        }

        if stack_size == 0 && game.enemy_player(self.index, destination).is_some() {
            return Err("the stack should contain at least one army unit".to_string());
        }

        if land_movement
            && self
                .get_units(destination)
                .iter()
                .filter(|unit| unit.unit_type.is_army_unit() && !unit.is_transported())
                .count()
                + stack_size
                > STACK_LIMIT
        {
            return Err("the destination stack limit would be exceeded".to_string());
        }
        Ok(())
    }

    #[must_use]
    pub fn get_unit(&self, id: u32) -> Option<&Unit> {
        self.units.iter().find(|unit| unit.id == id)
    }

    #[must_use]
    pub fn get_unit_mut(&mut self, id: u32) -> Option<&mut Unit> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    pub fn remove_unit(&mut self, id: u32) -> Option<Unit> {
        Some(
            self.units
                .remove(self.units.iter().position(|unit| unit.id == id)?),
        )
    }

    #[must_use]
    pub fn get_units(&self, position: Position) -> Vec<&Unit> {
        self.units
            .iter()
            .filter(|unit| unit.position == position)
            .collect()
    }

    #[must_use]
    pub fn get_units_mut(&mut self, position: Position) -> Vec<&mut Unit> {
        self.units
            .iter_mut()
            .filter(|unit| unit.position == position)
            .collect()
    }

    pub(crate) fn get_events(&self) -> &PlayerEvents {
        self.events.as_ref().expect("events should be set")
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if 'events' is set to None
    pub fn take_events<F>(&mut self, action: F)
    where
        F: FnOnce(&PlayerEvents, &mut Player),
    {
        let events = self.events.take().expect("events should be set");
        action(&events, self);
        self.events = Some(events);
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct PlayerData {
    name: Option<String>,
    id: usize,
    resources: ResourcePile,
    resource_limit: ResourcePile,
    cities: Vec<CityData>,
    units: Vec<Unit>,
    civilization: String,
    active_leader: Option<String>,
    available_leaders: Vec<String>,
    advances: Vec<String>,
    unlocked_special_advance: Vec<String>,
    wonders_build: Vec<String>,
    game_event_tokens: u8,
    completed_objectives: Vec<String>,
    captured_leaders: Vec<String>,
    event_victory_points: f32,
    wonder_cards: Vec<String>,
    collect_options: Vec<(Terrain, Vec<ResourcePile>)>,
    next_unit_id: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    played_once_per_turn_actions: Vec<CustomActionType>,
}
