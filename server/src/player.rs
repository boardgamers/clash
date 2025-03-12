use crate::action_card::ActionCard;
use crate::advance::Advance;
use crate::city_pieces::{DestroyedStructures, DestroyedStructuresData};
use crate::consts::{UNIT_LIMIT_BARBARIANS, UNIT_LIMIT_PIRATES};
use crate::content::advances::get_advance;
use crate::content::wonders::get_wonder;
use crate::content::{action_cards, builtin};
use crate::events::{Event, EventOrigin};
use crate::payment::PaymentOptions;
use crate::player_events::CostInfo;
use crate::resource::ResourceType;
use crate::unit::{carried_units, UnitData, UnitType};
use crate::{
    city::{City, CityData},
    city_pieces::Building::{self},
    civilization::Civilization,
    consts::{
        ADVANCE_COST, ADVANCE_VICTORY_POINTS, BUILDING_VICTORY_POINTS,
        CAPTURED_LEADER_VICTORY_POINTS, CITY_LIMIT, CITY_PIECE_LIMIT, CONSTRUCT_COST,
        OBJECTIVE_VICTORY_POINTS, UNIT_LIMIT, WONDER_VICTORY_POINTS,
    },
    content::{civilizations, custom_actions::CustomActionType},
    game::Game,
    leader::Leader,
    player_events::PlayerEvents,
    position::Position,
    resource_pile::ResourcePile,
    unit::{Unit, Units},
    utils,
    wonder::Wonder,
};
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    cmp::Ordering::{self, *},
    mem,
};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayerType {
    Human,
    Barbarian,
}

pub struct Player {
    name: Option<String>,
    pub index: usize,
    pub resources: ResourcePile,
    pub resource_limit: ResourcePile,
    // transient, only for the current turn, only the active player can gain resources
    pub wasted_resources: ResourcePile,
    pub(crate) events: PlayerEvents,
    pub cities: Vec<City>,
    pub destroyed_structures: DestroyedStructures,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<String>,
    pub available_leaders: Vec<String>,
    pub advances: Vec<Advance>,
    pub unlocked_special_advances: Vec<String>,
    pub wonders_build: Vec<String>,
    pub incident_tokens: u8,
    pub completed_objectives: Vec<String>,
    pub captured_leaders: Vec<String>,
    pub event_victory_points: f32,
    pub custom_actions: HashMap<CustomActionType, EventOrigin>,
    pub wonder_cards: Vec<String>,
    pub action_cards: Vec<ActionCard>,
    pub next_unit_id: u32,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub event_info: HashMap<String, String>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        let data = self.cloned_data();
        Self::from_data(data)
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.cloned_data() == other.cloned_data()
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
        builtin::init_player(game, player_index);
        let advances = mem::take(&mut game.players[player_index].advances);
        for advance in &advances {
            (advance.listeners.initializer)(game, player_index);
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
                    (special_advance.listeners.initializer)(game, player_index);
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
                (leader.listeners.initializer)(game, player_index);
            });
        }
        let mut cities = mem::take(&mut game.players[player_index].cities);
        for city in &mut cities {
            for wonder in &city.pieces.wonders {
                (wonder.listeners.initializer)(game, player_index);
            }
        }
        game.players[player_index].cities = cities;
        game.players[player_index].advances = advances;
    }

    fn from_data(data: PlayerData) -> Player {
        let units = data
            .units
            .into_iter()
            .flat_map(|u| Unit::from_data(data.id, u))
            .collect_vec();
        units
            .iter()
            .into_group_map_by(|unit| unit.id)
            .iter()
            .for_each(|(id, units)| {
                assert!(
                    units.len() == 1,
                    "player data {} should not contain duplicate units {id}",
                    data.id
                );
            });
        let player = Self {
            name: data.name,
            index: data.id,
            resources: data.resources,
            resource_limit: data.resource_limit,
            wasted_resources: ResourcePile::empty(),
            events: PlayerEvents::new(),
            cities: data
                .cities
                .into_iter()
                .map(|d| City::from_data(d, data.id))
                .collect(),
            destroyed_structures: DestroyedStructures::from_data(&data.destroyed_structures),
            units,
            civilization: civilizations::get_civilization(&data.civilization)
                .expect("player data should have a valid civilization"),
            active_leader: data.active_leader,
            available_leaders: data.available_leaders,
            advances: data.advances.iter().map(|a| get_advance(a)).collect(),
            unlocked_special_advances: data.unlocked_special_advance,
            wonders_build: data.wonders_build,
            incident_tokens: data.incident_tokens,
            completed_objectives: data.completed_objectives,
            captured_leaders: data.captured_leaders,
            event_victory_points: data.event_victory_points,
            custom_actions: HashMap::new(),
            wonder_cards: data.wonder_cards,
            action_cards: data
                .action_cards
                .iter()
                .map(|action_card| action_cards::get_action_card(*action_card))
                .collect(),
            next_unit_id: data.next_unit_id,
            played_once_per_turn_actions: data.played_once_per_turn_actions,
            event_info: data.event_info,
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
            destroyed_structures: self.destroyed_structures.data(),
            units,
            civilization: self.civilization.name,
            active_leader: self.active_leader,
            available_leaders: self.available_leaders.into_iter().collect(),
            advances: self.advances.into_iter().map(|a| a.name).sorted().collect(),
            unlocked_special_advance: self.unlocked_special_advances,
            wonders_build: self.wonders_build,
            incident_tokens: self.incident_tokens,
            completed_objectives: self.completed_objectives,
            captured_leaders: self.captured_leaders,
            event_victory_points: self.event_victory_points,
            wonder_cards: self.wonder_cards,
            action_cards: self
                .action_cards
                .into_iter()
                .map(|action_card| action_card.id)
                .collect(),
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions,
            event_info: self.event_info,
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
            destroyed_structures: self.destroyed_structures.cloned_data(),
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
            incident_tokens: self.incident_tokens,
            completed_objectives: self.completed_objectives.clone(),
            captured_leaders: self.captured_leaders.clone(),
            event_victory_points: self.event_victory_points,
            wonder_cards: self.wonder_cards.clone(),
            action_cards: self
                .action_cards
                .iter()
                .map(|action_card| action_card.id)
                .collect(),
            next_unit_id: self.next_unit_id,
            played_once_per_turn_actions: self.played_once_per_turn_actions.clone(),
            event_info: self.event_info.clone(),
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
            resources: ResourcePile::empty(),
            resource_limit: ResourcePile::empty(),
            wasted_resources: ResourcePile::empty(),
            events: PlayerEvents::new(),
            cities: Vec::new(),
            destroyed_structures: DestroyedStructures::new(),
            units: Vec::new(),
            active_leader: None,
            available_leaders: civilization
                .leaders
                .iter()
                .map(|l| l.name.clone())
                .collect(),
            civilization,
            advances: vec![],
            unlocked_special_advances: Vec::new(),
            incident_tokens: 0,
            completed_objectives: Vec::new(),
            captured_leaders: Vec::new(),
            event_victory_points: 0.0,
            custom_actions: HashMap::new(),
            wonder_cards: Vec::new(),
            action_cards: Vec::new(),
            wonders_build: Vec::new(),
            next_unit_id: 0,
            played_once_per_turn_actions: Vec::new(),
            event_info: HashMap::new(),
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
        self.event_info.clear();
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    #[must_use]
    pub fn get_name(&self) -> String {
        if self.is_human() {
            self.name
                .clone()
                .unwrap_or(format!("Player{}", self.index + 1))
        } else {
            self.civilization.name.clone()
        }
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
    pub fn can_afford(&self, cost: &PaymentOptions) -> bool {
        cost.can_afford(&self.resources)
    }

    pub(crate) fn pay_cost(&mut self, cost: &PaymentOptions, payment: &ResourcePile) {
        assert!(cost.can_afford(payment), "invalid payment");
        assert!(cost.is_valid_payment(payment), "Invalid payment");
        self.lose_resources(payment.clone());
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if player cannot afford the resources
    pub(crate) fn lose_resources(&mut self, resources: ResourcePile) {
        assert!(
            self.resources.has_at_least(&resources),
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
        self.can_afford(&self.advance_cost(advance, None).cost) && self.can_advance_free(advance)
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
            < CITY_PIECE_LIMIT - self.destroyed_structures.get_building(building)
    }

    #[must_use]
    pub fn is_city_available(&self) -> bool {
        self.cities.len() < (CITY_LIMIT - self.destroyed_structures.cities) as usize
    }

    #[must_use]
    pub fn is_human(&self) -> bool {
        self.civilization.is_human()
    }

    #[must_use]
    pub fn available_units(&self) -> Units {
        let mut units = if self.is_human() {
            UNIT_LIMIT.clone()
        } else if self.civilization.is_barbarian() {
            UNIT_LIMIT_BARBARIANS.clone()
        } else {
            UNIT_LIMIT_PIRATES.clone()
        };
        for u in &self.units {
            units -= &u.unit_type;
        }
        units
    }

    pub fn remove_wonder(&mut self, wonder: &Wonder) {
        utils::remove_element(&mut self.wonders_build, &wonder.name);
    }

    #[must_use]
    pub fn incident_tokens(&self) -> u8 {
        self.incident_tokens
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = Vec::new();
        self.action_cards = Vec::new();
        //replace with empty strings to see the number of cards?
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
    pub fn construct_cost(
        &self,
        building: Building,
        city: &City,
        execute: Option<&ResourcePile>,
    ) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.construct_cost,
            &PaymentOptions::resources(CONSTRUCT_COST),
            city,
            &building,
            execute,
        )
    }

    #[must_use]
    pub fn wonder_cost(
        &self,
        wonder: &Wonder,
        city: &City,
        execute: Option<&ResourcePile>,
    ) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.wonder_cost,
            &wonder.cost.clone(),
            city,
            wonder,
            execute,
        )
    }

    #[must_use]
    pub fn increase_happiness_cost(&self, city: &City, steps: u32) -> Option<CostInfo> {
        let max_steps = 2 - city.mood_state.clone() as u32;
        let cost = city.size() as u32 * steps;
        if steps > max_steps {
            None
        } else {
            Some(self.increase_happiness_total_cost(cost, None))
        }
    }

    #[must_use]
    pub(crate) fn increase_happiness_total_cost(
        &self,
        cost: u32,
        execute: Option<&ResourcePile>,
    ) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.happiness_cost,
            &PaymentOptions::sum(cost, &[ResourceType::MoodTokens]),
            &(),
            &(),
            execute,
        )
    }

    #[must_use]
    pub fn advance_cost(&self, advance: &Advance, execute: Option<&ResourcePile>) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.advance_cost,
            &PaymentOptions::sum(
                ADVANCE_COST,
                &[ResourceType::Ideas, ResourceType::Food, ResourceType::Gold],
            ),
            advance,
            &(),
            execute,
        )
    }

    ///
    /// # Panics
    /// Panics if city does not exist
    #[must_use]
    pub fn get_city(&self, position: Position) -> &City {
        self.try_get_city(position).expect("city should exist")
    }

    #[must_use]
    pub fn try_get_city(&self, position: Position) -> Option<&City> {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)?;
        Some(&self.cities[position])
    }

    ///
    /// # Panics
    /// Panics if city does not exist
    #[must_use]
    pub fn get_city_mut(&mut self, position: Position) -> &mut City {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)
            .expect("city should exist");
        &mut self.cities[position]
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
                .try_get_city(city_position)
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
        let index = self.index;
        let city = self.get_city_mut(city_position);
        city.activate();
        city.pieces.set_building(building, index);
        if let Some(port_position) = port_position {
            city.port_position = Some(port_position);
        }
    }

    pub fn add_unit(&mut self, position: Position, unit_type: UnitType) {
        let unit = Unit::new(self.index, position, unit_type, self.next_unit_id);
        self.units.push(unit);
        self.next_unit_id += 1;
    }

    ///
    /// # Panics
    /// Panics if unit does not exist
    #[must_use]
    pub fn get_unit(&self, id: u32) -> &Unit {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .expect("unit should exist")
    }

    ///
    /// # Panics
    /// Panics if unit does not exist
    #[must_use]
    pub fn get_unit_mut(&mut self, id: u32) -> &mut Unit {
        self.units
            .iter_mut()
            .find(|unit| unit.id == id)
            .expect("unit should exist")
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

    #[must_use]
    pub(crate) fn trigger_event<T, U, V>(
        &self,
        event: fn(&PlayerEvents) -> &Event<T, U, V>,
        value: &mut T,
        info: &U,
        details: &V,
    ) -> Vec<EventOrigin>
    where
        T: Clone + PartialEq,
    {
        let e = event(&self.events);
        e.get().trigger(value, info, details)
    }

    pub(crate) fn trigger_cost_event<U, V>(
        &self,
        get_event: fn(&PlayerEvents) -> &Event<CostInfo, U, V>,
        value: &PaymentOptions,
        info: &U,
        details: &V,
        execute: Option<&ResourcePile>,
    ) -> CostInfo {
        let event = get_event(&self.events).get();
        let mut cost_info = CostInfo::new(self, value.clone());
        if let Some(execute) = execute {
            event.trigger_with_minimal_modifiers(
                &cost_info,
                info,
                details,
                |i| i.cost.is_valid_payment(execute),
                |i, m| i.cost.modifiers = m,
            )
        } else {
            let m = event.trigger(&mut cost_info, info, details);
            cost_info.cost.modifiers = m;
            cost_info
        }
    }

    pub(crate) fn trigger_player_event<U, V>(
        &mut self,
        event: fn(&mut PlayerEvents) -> &mut Event<Player, U, V>,
        info: &U,
        details: &V,
    ) {
        let e = event(&mut self.events).take();
        let _ = e.trigger(self, info, details);
        event(&mut self.events).set(e);
    }

    #[must_use]
    pub fn wonder_cards(&self) -> Vec<Wonder> {
        self.wonder_cards.iter().map(|n| get_wonder(n)).collect()
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct PlayerData {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    id: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    resources: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    resource_limit: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cities: Vec<CityData>,
    #[serde(default)]
    #[serde(skip_serializing_if = "DestroyedStructuresData::is_empty")]
    destroyed_structures: DestroyedStructuresData,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    units: Vec<UnitData>,
    civilization: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    active_leader: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    available_leaders: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    advances: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unlocked_special_advance: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonders_build: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    incident_tokens: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    completed_objectives: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    captured_leaders: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "f32::is_zero")]
    event_victory_points: f32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonder_cards: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_cards: Vec<u8>,
    next_unit_id: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    played_once_per_turn_actions: Vec<CustomActionType>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    event_info: HashMap<String, String>,
}
