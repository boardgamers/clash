use crate::advance::Advance;
use crate::content::advances::{get_advance_by_name, RITUALS};
use crate::game::CurrentMove;
use crate::game::GameState::Movement;
use crate::movement::move_routes;
use crate::movement::{is_valid_movement_type, MoveRoute};
use crate::payment::PaymentModel;
use crate::resource::ResourceType;
use crate::unit::{carried_units, get_current_move, MovementRestriction, UnitData};
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
    player_events::PlayerEvents,
    position::Position,
    resource_pile::ResourcePile,
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
    collections::HashSet,
    mem,
};

pub struct Player {
    name: Option<String>,
    pub index: usize,
    pub resources: ResourcePile,
    pub resource_limit: ResourcePile,
    // transient, only for the current turn, only the active player can gain resources
    pub wasted_resources: ResourcePile,
    pub(crate) events: Option<PlayerEvents>,
    pub cities: Vec<City>,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<String>,
    pub available_leaders: Vec<String>,
    pub advances: Vec<Advance>,
    pub unlocked_special_advances: Vec<String>,
    pub wonders_build: Vec<String>,
    pub game_event_tokens: u8,
    pub completed_objectives: Vec<String>,
    pub captured_leaders: Vec<String>,
    pub event_victory_points: f32,
    pub custom_actions: HashSet<CustomActionType>,
    pub wonder_cards: Vec<Wonder>,
    pub next_unit_id: u32,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub played_once_per_turn_effects: Vec<String>,
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
            && self
                .units
                .iter()
                .enumerate()
                .all(|(i, unit)| unit.id == other.units[i].id)
            && self.civilization.name == other.civilization.name
            && self.active_leader == other.active_leader
            && self
                .available_leaders
                .iter()
                .enumerate()
                .all(|(i, leader)| *leader == other.available_leaders[i])
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
    }
}

impl Player {
    ///
    ///
    /// # Panics
    ///
    /// Panics if elements like wonders or advances don't exist
    pub fn initialize_player(data: PlayerData, game: &mut Game) {
        let leader = data.active_leader.clone();
        let player = Self::from_data(data);
        let player_index = player.index;
        game.players.push(player);
        let advances = mem::take(&mut game.players[player_index].advances);
        for advance in &advances {
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
        if let Some(leader) = leader {
            Self::with_leader(&leader, game, player_index, |game, leader| {
                (leader.player_initializer)(game, player_index);
            });
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
        let units: Vec<_> = data
            .units
            .into_iter()
            .flat_map(|u| Unit::from_data(data.id, u))
            .collect();
        units
            .iter()
            .into_group_map_by(|unit| unit.id)
            .iter()
            .for_each(|(id, units)| {
                assert!(
                    units.len() == 1,
                    "player data should not contain duplicate units {id}"
                );
            });
        let player = Self {
            name: data.name,
            index: data.id,
            resources: data.resources,
            resource_limit: data.resource_limit,
            wasted_resources: ResourcePile::empty(),
            events: Some(PlayerEvents::default()),
            cities: data
                .cities
                .into_iter()
                .map(|d| City::from_data(d, data.id))
                .collect(),
            units,
            civilization: civilizations::get_civilization_by_name(&data.civilization)
                .expect("player data should have a valid civilization"),
            active_leader: data.active_leader,
            available_leaders: data.available_leaders,
            advances: data
                .advances
                .iter()
                .map(|a| get_advance_by_name(a))
                .collect(),
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
            next_unit_id: data.next_unit_id,
            played_once_per_turn_actions: data.played_once_per_turn_actions,
            played_once_per_turn_effects: data.played_once_per_turn_effects,
        };
        player
    }

    #[must_use]
    pub fn data(self) -> PlayerData {
        let units = self
            .units
            .iter()
            // carried units are added to carriers
            .filter(|unit| unit.carrier_id.is_none())
            .sorted_by_key(|unit| unit.id)
            .map(|u| u.data(&self))
            .collect();
        PlayerData {
            name: self.name,
            id: self.index,
            resources: self.resources,
            resource_limit: self.resource_limit,
            cities: self.cities.into_iter().map(City::data).collect(),
            units,
            civilization: self.civilization.name,
            active_leader: self.active_leader,
            available_leaders: self.available_leaders.into_iter().collect(),
            advances: self.advances.into_iter().map(|a| a.name).sorted().collect(),
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
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
            played_once_per_turn_effects: self.played_once_per_turn_effects,
        }
    }

    pub fn cloned_data(&self) -> PlayerData {
        let units = self
            .units
            .iter()
            // carried units are added to carriers
            .filter(|unit| unit.carrier_id.is_none())
            .sorted_by_key(|unit| unit.id)
            .map(|u| u.data(self))
            .collect();
        PlayerData {
            name: self.name.clone(),
            id: self.index,
            resources: self.resources.clone(),
            resource_limit: self.resource_limit.clone(),
            cities: self.cities.iter().map(City::cloned_data).collect(),
            units,
            civilization: self.civilization.name.clone(),
            active_leader: self.active_leader.clone(),
            available_leaders: self.available_leaders.clone(),
            advances: self
                .advances
                .iter()
                .map(|a| a.name.clone())
                .sorted()
                .collect(),
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
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions.clone(),
            played_once_per_turn_effects: self.played_once_per_turn_effects.clone(),
        }
    }

    ///
    /// # Panics
    /// Panics if the civilization does not exist
    #[must_use]
    pub fn new(civilization: Civilization, index: usize) -> Self {
        Self {
            name: None,
            index,
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(2, 7, 7, 7, 7, 0, 0),
            wasted_resources: ResourcePile::empty(),
            events: Some(PlayerEvents::new()),
            cities: Vec::new(),
            units: Vec::new(),
            active_leader: None,
            available_leaders: civilization
                .leaders
                .iter()
                .map(|l| l.name.clone())
                .collect(),
            civilization,
            advances: vec![
                advances::get_advance_by_name("Farming"),
                advances::get_advance_by_name("Mining"),
            ],
            unlocked_special_advances: Vec::new(),
            game_event_tokens: 3,
            completed_objectives: Vec::new(),
            captured_leaders: Vec::new(),
            event_victory_points: 0.0,
            custom_actions: HashSet::new(),
            wonder_cards: Vec::new(),
            wonders_build: Vec::new(),
            next_unit_id: 0,
            played_once_per_turn_actions: Vec::new(),
            played_once_per_turn_effects: Vec::new(),
        }
    }

    #[must_use]
    pub fn active_leader(&self) -> Option<&Leader> {
        self.active_leader
            .as_ref()
            .and_then(|name| self.get_leader(name))
    }

    #[must_use]
    pub fn get_leader(&self, name: &String) -> Option<&Leader> {
        self.civilization
            .leaders
            .iter()
            .find(|leader| &leader.name == name)
    }

    pub(crate) fn with_leader(
        leader: &str,
        game: &mut Game,
        player_index: usize,
        f: impl FnOnce(&mut Game, &Leader),
    ) {
        let pos = game.players[player_index]
            .civilization
            .leaders
            .iter()
            .position(|l| l.name == leader)
            .expect("player should have the leader");
        let l = game.players[player_index].civilization.leaders.remove(pos);
        f(game, &l);
        game.players[player_index]
            .civilization
            .leaders
            .insert(pos, l);
    }

    #[must_use]
    pub fn available_leaders(&self) -> Vec<&Leader> {
        self.available_leaders
            .iter()
            .filter_map(|name| self.get_leader(name))
            .collect()
    }

    pub fn end_turn(&mut self) {
        for city in &mut self.cities {
            city.deactivate();
        }
        for unit in &mut self.units {
            unit.movement_restrictions = vec![];
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
            .find_map(|advance| advance.government.clone())
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.resources += resources;
        let waste = self.resources.apply_resource_limit(&self.resource_limit);
        self.wasted_resources += waste;
    }

    #[must_use]
    pub fn can_afford_resources(&self, cost: &ResourcePile) -> bool {
        self.can_afford(&PaymentModel::resources(cost.clone()))
    }

    #[must_use]
    pub fn can_afford(&self, cost: &PaymentModel) -> bool {
        cost.can_afford(&self.resources)
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if player cannot afford the resources
    pub fn loose_resources(&mut self, resources: ResourcePile) {
        assert!(
            self.can_afford_resources(&resources),
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
        self.can_afford(&self.advance_cost(&advance.name)) && self.can_advance_free(advance)
    }

    #[must_use]
    pub fn has_advance(&self, advance: &str) -> bool {
        self.advances.iter().any(|a| a.name == advance)
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

        match sum
            .partial_cmp(&other_sum)
            .expect("should be able to compare")
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }

        for (part, other_part) in parts.iter().zip(other_parts.iter()) {
            match part
                .partial_cmp(other_part)
                .expect("should be able to compare")
            {
                Less => return Less,
                Equal => (),
                Greater => return Greater,
            }
        }
        Equal
    }

    #[must_use]
    pub fn construct_cost(&self, building: Building, city: &City) -> PaymentModel {
        let mut cost = CONSTRUCT_COST;
        self.get_events()
            .construct_cost
            .trigger(&mut cost, city, &building);
        PaymentModel::resources(cost)
    }

    #[must_use]
    pub fn wonder_cost(&self, wonder: &Wonder, city: &City) -> PaymentModel {
        let mut cost = wonder.cost.clone();
        self.get_events()
            .wonder_cost
            .trigger(&mut cost, city, wonder);
        cost
    }

    #[must_use]
    pub fn increase_happiness_cost(&self, city: &City, steps: u32) -> Option<PaymentModel> {
        let max_steps = 2 - city.mood_state.clone() as u32;
        let cost = city.size() as u32 * steps;
        if steps > max_steps {
            None
        } else if self.has_advance(RITUALS) {
            Some(PaymentModel::sum(
                cost,
                &[
                    ResourceType::Food,
                    ResourceType::Wood,
                    ResourceType::Ore,
                    ResourceType::Ideas,
                    ResourceType::MoodTokens,
                    ResourceType::Gold,
                ],
            ))
        } else {
            Some(PaymentModel::sum(cost, &[ResourceType::MoodTokens]))
        }
    }

    #[must_use]
    pub fn advance_cost(&self, advance: &str) -> PaymentModel {
        let mut cost = ADVANCE_COST;
        self.get_events()
            .advance_cost
            .trigger(&mut cost, &advance.to_string(), &());
        PaymentModel::sum(
            cost,
            &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
        )
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
        leader_name: Option<&String>,
        replaced_units: &[u32],
    ) -> bool {
        if !self.can_recruit_without_replaced(units, city_position, leader_name) {
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
        leader_name: Option<&String>,
    ) -> bool {
        let city = self
            .get_city(city_position)
            .expect("player should have a city at the recruitment position");
        if !city.can_activate() {
            return false;
        }
        let cost = PaymentModel::resources(units.iter().map(UnitType::cost).sum());
        if !self.can_afford(&cost) {
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

        let leaders = units
            .iter()
            .filter(|unit| matches!(unit, UnitType::Leader))
            .count();
        match leaders {
            0 => leader_name.is_none(),
            1 => leader_name.is_some_and(|n| self.available_leaders.contains(n)),
            _ => false,
        }
    }

    pub fn add_unit(&mut self, position: Position, unit_type: UnitType) {
        let unit = Unit::new(self.index, position, unit_type, self.next_unit_id);
        self.units.push(unit);
        self.next_unit_id += 1;
    }

    /// # Errors
    ///
    /// Will return `Err` if the unit cannot move.
    ///
    /// # Panics
    ///
    /// Panics if destination tile does not exist
    pub fn move_units_destinations(
        &self,
        game: &Game,
        unit_ids: &[u32],
        start: Position,
        embark_carrier_id: Option<u32>,
    ) -> Result<Vec<MoveRoute>, String> {
        let (moved_units, movement_actions_left, current_move) = if let Movement(m) = &game.state {
            (&m.moved_units, m.movement_actions_left, &m.current_move)
        } else {
            (&vec![], 1, &CurrentMove::None)
        };

        let units = unit_ids
            .iter()
            .map(|id| {
                self.get_unit(*id)
                    .expect("the player should have all units to move")
            })
            .collect::<Vec<_>>();

        if units.is_empty() {
            return Err("noun units to move".to_string());
        }
        if embark_carrier_id.is_some_and(|id| {
            let player_index = self.index;
            carried_units(id, &game.players[player_index]).len() + units.len() > 2
        }) {
            return Err("carrier capacity exceeded".to_string());
        }

        let carrier_position = embark_carrier_id.map(|id| {
            self.get_unit(id)
                .expect("the player should have the carrier unit")
                .position
        });

        let mut stack_size = 0;
        let mut movement_restrictions = vec![];

        for unit in &units {
            if unit.position != start {
                return Err("the unit should be at the starting position".to_string());
            }
            movement_restrictions.extend(unit.movement_restrictions.iter());
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
            }
            if unit.unit_type.is_army_unit() && !self.has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE) {
                return Err("army movement advance missing".to_string());
            }
            if unit.unit_type.is_army_unit() && !unit.unit_type.is_settler() {
                stack_size += 1;
            }
        }

        let destinations: Vec<MoveRoute> =
            move_routes(start, self, unit_ids, game, embark_carrier_id)
                .iter()
                .filter(|route| {
                    if !self.can_afford_resources(&route.cost) {
                        return false;
                    }
                    if movement_restrictions.contains(&&MovementRestriction::Battle) {
                        return false;
                    }
                    let dest = route.destination;
                    let attack = game.enemy_player(self.index, dest).is_some();
                    if attack && stack_size == 0 {
                        return false;
                    }

                    if !route.ignore_terrain_movement_restrictions {
                        if movement_restrictions
                            .iter()
                            .contains(&&MovementRestriction::Mountain)
                        {
                            return false;
                        }
                        if attack
                            && movement_restrictions
                                .iter()
                                .contains(&&MovementRestriction::Forest)
                        {
                            return false;
                        }
                    }

                    if game.map.is_land(start)
                        && self
                            .get_units(dest)
                            .iter()
                            .filter(|unit| unit.unit_type.is_army_unit() && !unit.is_transported())
                            .count()
                            + stack_size
                            + route.stack_size_used
                            > STACK_LIMIT
                    {
                        return false;
                    }

                    if !is_valid_movement_type(game, &units, carrier_position, dest) {
                        return false;
                    }

                    if !matches!(current_move, CurrentMove::None)
                        && *current_move
                            == get_current_move(game, unit_ids, start, dest, embark_carrier_id)
                    {
                        return true;
                    }

                    if movement_actions_left == 0 {
                        return false;
                    }

                    if unit_ids.iter().any(|id| moved_units.contains(id)) {
                        return false;
                    }
                    true
                })
                .cloned()
                .collect();

        if destinations.is_empty() {
            return Err("no valid destinations".to_string());
        }
        Ok(destinations)
    }

    #[must_use]
    pub fn get_unit(&self, id: u32) -> Option<&Unit> {
        self.units.iter().find(|unit| unit.id == id)
    }

    #[must_use]
    pub fn get_unit_mut(&mut self, id: u32) -> Option<&mut Unit> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    pub(crate) fn remove_unit(&mut self, id: u32) -> Unit {
        for id in carried_units(id, self) {
            self.remove_unit(id);
        }

        self.units.remove(
            self.units
                .iter()
                .position(|unit| unit.id == id)
                .expect("unit should exist"),
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
    pub(crate) fn take_events<F>(&mut self, action: F)
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
    units: Vec<UnitData>,
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
    next_unit_id: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    played_once_per_turn_actions: Vec<CustomActionType>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    played_once_per_turn_effects: Vec<String>,
}
