use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering::{self, *},
    collections::{HashMap, HashSet},
    mem,
};

use crate::{
    city::{City, CityData},
    city_pieces::{
        AvailableBuildings,
        Building::{self, *},
    },
    civilization::Civilization,
    content::{advances, civilizations, custom_actions::CustomActionType, wonders},
    game::Game,
    leader::Leader,
    map::Terrain::{self, *},
    player_events::PlayerEvents,
    position::Position,
    resource_pile::{AdvancePaymentOptions, ResourcePile},
    unit::{MovementRestriction, Unit},
    wonder::Wonder,
};

pub const CONSTRUCT_COST: ResourcePile = ResourcePile {
    food: 1,
    wood: 1,
    ore: 1,
    ideas: 0,
    gold: 0,
    mood_tokens: 0,
    culture_tokens: 0,
};

const ADVANCE_COST: u32 = 2;
const BUILDING_VICTORY_POINTS: f32 = 1.0;
const ADVANCE_VICTORY_POINTS: f32 = 0.5;
const OBJECTIVE_VICTORY_POINTS: f32 = 2.0;
const WONDER_VICTORY_POINTS: f32 = 4.0;
const DEFEATED_LEADER_VICTORY_POINTS: f32 = 2.0;

pub struct Player {
    name: Option<String>,
    pub index: usize,
    resources: ResourcePile,
    pub resource_limit: ResourcePile,
    pub events: Option<PlayerEvents>,
    pub cities: Vec<City>,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub advances: Vec<String>,
    pub unlocked_special_advances: Vec<String>,
    pub wonders: Vec<String>,
    pub wonders_build: usize,
    pub leader_position: Option<Position>,
    pub influenced_buildings: u32,
    pub game_event_tokens: u8,
    pub completed_objectives: Vec<String>,
    pub defeated_leaders: Vec<String>,
    pub event_victory_points: f32,
    pub custom_actions: HashSet<CustomActionType>,
    pub wonder_cards: Vec<Wonder>,
    pub available_buildings: AvailableBuildings,
    pub collect_options: HashMap<Terrain, Vec<ResourcePile>>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        let data = self.cloned_data();
        Self::from_data_uninitialized(data)
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
            && self.wonders == other.wonders
            && self.wonders_build == other.wonders_build
            && self.leader_position == other.leader_position
            && self.game_event_tokens == other.game_event_tokens
            && self.influenced_buildings == other.influenced_buildings
            && self.completed_objectives == other.completed_objectives
            && self.defeated_leaders == other.defeated_leaders
            && self.event_victory_points == other.event_victory_points
            && self
                .wonder_cards
                .iter()
                .enumerate()
                .all(|(i, wonder)| wonder.name == other.wonder_cards[i].name)
            && self.available_buildings == other.available_buildings
            && self.collect_options == other.collect_options
    }
}

impl Player {
    pub fn from_data(data: PlayerData, game: &mut Game) -> Self {
        let player = Self::from_data_uninitialized(data);
        let player_index = player.index;
        game.players.push(player);
        let advances = mem::take(&mut game.players[player_index].advances);
        for advance in advances.iter() {
            let advance = advances::get_advance_by_name(advance).expect("advance should exist");
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
        for city in cities.iter_mut() {
            for wonder in city.city_pieces.wonders.iter() {
                (wonder.player_initializer)(game, player_index);
            }
        }
        let mut player = game.players.remove(player_index);
        player.cities = cities;
        player.advances = advances;
        player
    }

    fn from_data_uninitialized(data: PlayerData) -> Player {
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
            wonders: data.wonders,
            wonders_build: data.wonders_build,
            leader_position: data.leader_position,
            game_event_tokens: data.game_event_tokens,
            influenced_buildings: data.influenced_buildings,
            completed_objectives: data.completed_objectives,
            defeated_leaders: data.defeated_leaders,
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
            available_buildings: data.available_buildings,
            collect_options: data
                .collect_options
                .into_iter()
                .map(|(terrain, options)| (terrain, options.into_iter().collect()))
                .collect(),
        };
        player
    }

    pub fn data(self) -> PlayerData {
        PlayerData {
            name: self.name,
            id: self.index,
            resources: self.resources,
            resource_limit: self.resource_limit,
            cities: self.cities.into_iter().map(|city| city.data()).collect(),
            units: self.units,
            civilization: self.civilization.name,
            active_leader: self.active_leader.map(|leader| leader.name),
            available_leaders: self
                .available_leaders
                .into_iter()
                .map(|leader| leader.name)
                .collect(),
            advances: self.advances,
            unlocked_special_advance: self.unlocked_special_advances,
            wonders: self.wonders,
            wonders_build: self.wonders_build,
            leader_position: self.leader_position,
            game_event_tokens: self.game_event_tokens,
            influenced_buildings: self.influenced_buildings,
            completed_objectives: self.completed_objectives,
            defeated_leaders: self.defeated_leaders,
            event_victory_points: self.event_victory_points,
            wonder_cards: self
                .wonder_cards
                .into_iter()
                .map(|wonder| wonder.name)
                .collect(),
            available_buildings: self.available_buildings,
            collect_options: self
                .collect_options
                .into_iter()
                .map(|(terrain, options)| (terrain, options.into_iter().collect()))
                .collect(),
        }
    }

    fn cloned_data(&self) -> PlayerData {
        PlayerData {
            name: self.name.clone(),
            id: self.index,
            resources: self.resources.clone(),
            resource_limit: self.resource_limit.clone(),
            cities: self.cities.iter().map(|city| city.cloned_data()).collect(),
            units: self.units.clone(),
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
            advances: self.advances.clone(),
            unlocked_special_advance: self.unlocked_special_advances.clone(),
            wonders: self.wonders.clone(),
            wonders_build: self.wonders_build,
            leader_position: self.leader_position.clone(),
            game_event_tokens: self.game_event_tokens,
            influenced_buildings: self.influenced_buildings,
            completed_objectives: self.completed_objectives.clone(),
            defeated_leaders: self.defeated_leaders.clone(),
            event_victory_points: self.event_victory_points,
            wonder_cards: self
                .wonder_cards
                .iter()
                .map(|wonder| wonder.name.clone())
                .collect(),
            available_buildings: self.available_buildings.clone(),
            collect_options: self
                .collect_options
                .iter()
                .map(|(terrain, options)| (terrain.clone(), options.to_vec()))
                .collect(),
        }
    }

    pub fn new(civilization: Civilization, index: usize) -> Self {
        Self {
            name: None,
            index,
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(2, 7, 7, 7, 7, 7, 7),
            events: Some(PlayerEvents::new()),
            cities: Vec::new(),
            units: Vec::new(),
            civilization,
            active_leader: None,
            available_leaders: Vec::new(),
            advances: Vec::new(),
            unlocked_special_advances: Vec::new(),
            wonders: Vec::new(),
            wonders_build: 0,
            leader_position: None,
            game_event_tokens: 3,
            influenced_buildings: 0,
            completed_objectives: Vec::new(),
            defeated_leaders: Vec::new(),
            event_victory_points: 0.0,
            custom_actions: HashSet::new(),
            wonder_cards: Vec::new(),
            available_buildings: AvailableBuildings::new(5, 5, 5, 5, 5, 5, 5),
            collect_options: HashMap::from([
                (Mountain, vec![ResourcePile::ore(1)]),
                (Fertile, vec![ResourcePile::food(1)]),
                (Forest, vec![ResourcePile::wood(1)]),
            ]),
        }
    }

    pub fn end_turn(&mut self) {
        for city in self.cities.iter_mut() {
            city.deactivate();
        }
        for unit in self.units.iter_mut() {
            unit.movement_restriction = MovementRestriction::None;
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn get_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or(format!("Player{}", self.index + 1))
    }

    pub fn government(&self) -> Option<String> {
        self.advances
            .iter()
            .filter_map(|advance| {
                advances::get_advance_by_name(advance)
                    .expect("all player owned advances should exist")
                    .government
            })
            .next()
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.resources += resources;
        self.resources.apply_resource_limit(&self.resource_limit);
    }

    pub fn loose_resources(&mut self, resources: ResourcePile) {
        self.resources -= resources;
    }

    pub fn resources(&self) -> &ResourcePile {
        &self.resources
    }

    pub fn can_advance_free(&self, advance: &str) -> bool {
        let advance = advances::get_advance_by_name(advance).expect("advance should exist");
        if self.has_advance(&advance.name) {
            return false;
        }
        if let Some(required_advance) = &advance.required_advance {
            if !self.has_advance(required_advance) {
                return false;
            }
        }
        if let Some(contradicting_advance) = &advance.contradicting_advance {
            if self.has_advance(contradicting_advance) {
                return false;
            }
        }
        true
    }

    pub fn can_advance(&self, advance: &str) -> bool {
        if self.resources.food + self.resources.ideas + (self.resources.gold as u32)
            < self.advance_cost(advance)
        {
            return false;
        }
        self.can_advance_free(advance)
    }

    pub fn has_advance(&self, advance: &str) -> bool {
        self.advances.iter().any(|advances| advances == advance)
    }

    pub fn victory_points(&self) -> f32 {
        let mut victory_points = 0.0;
        for city in self.cities.iter() {
            victory_points += city.uninfluenced_buildings() as f32 * BUILDING_VICTORY_POINTS;
            victory_points += 1.0;
        }
        victory_points += self.influenced_buildings as f32 * BUILDING_VICTORY_POINTS;
        victory_points += (self.advances.len() + self.unlocked_special_advances.len()) as f32
            * ADVANCE_VICTORY_POINTS;
        victory_points += self.completed_objectives.len() as f32 * OBJECTIVE_VICTORY_POINTS;
        victory_points += self.wonders.len() as f32 * WONDER_VICTORY_POINTS / 2.0;
        victory_points += self.wonders_build as f32 * WONDER_VICTORY_POINTS / 2.0;
        victory_points += self.event_victory_points;
        victory_points += self.defeated_leaders.len() as f32 * DEFEATED_LEADER_VICTORY_POINTS;
        victory_points
    }

    pub fn remove_wonder(&mut self, wonder: &Wonder) {
        self.wonders.remove(
            self.wonders
                .iter()
                .position(|player_wonder| player_wonder == &wonder.name)
                .expect("player should have wonder"),
        );
    }

    pub fn game_event_tokens(&self) -> u8 {
        self.game_event_tokens
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = Vec::new();
        //todo strip information about other hand cards
    }

    pub fn compare_score(&self, other: &Self) -> Ordering {
        let mut building_score = 0;
        for city in self.cities.iter() {
            building_score += city.uninfluenced_buildings();
        }
        building_score += self.influenced_buildings;
        let mut other_building_score = 0;
        for city in self.cities.iter() {
            other_building_score += city.uninfluenced_buildings();
        }
        other_building_score += self.influenced_buildings;
        match building_score.cmp(&other_building_score) {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
        match (self.advances.len() + self.unlocked_special_advances.len())
            .cmp(&(other.advances.len() + other.unlocked_special_advances.len()))
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
        match self
            .completed_objectives
            .len()
            .cmp(&other.completed_objectives.len())
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
        match (self.wonders.len() + self.wonders_build)
            .cmp(&(other.wonders.len() + other.wonders_build))
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }
        self.event_victory_points
            .total_cmp(&other.event_victory_points)
    }

    pub fn construct_cost(&self, building: &Building, city: &City) -> ResourcePile {
        let mut cost = CONSTRUCT_COST;
        self.get_events()
            .construct_cost
            .trigger(&mut cost, city, building);
        cost
    }

    pub fn wonder_cost(&self, wonder: &Wonder, city: &City) -> ResourcePile {
        let mut cost = wonder.cost.clone();
        self.get_events()
            .wonder_cost
            .trigger(&mut cost, city, wonder);
        cost
    }

    pub fn advance_cost(&self, advance: &str) -> u32 {
        let mut cost = ADVANCE_COST;
        self.get_events()
            .advance_cost
            .trigger(&mut cost, &advance.to_string(), &());
        cost
    }

    pub fn get_advance_payment_options(&self, advance: &str) -> AdvancePaymentOptions {
        self.resources()
            .get_advance_payment_options(self.advance_cost(advance))
    }

    pub fn get_city(&self, position: &Position) -> Option<&City> {
        let position = self
            .cities
            .iter()
            .position(|city| &city.position == position)?;
        Some(&self.cities[position])
    }

    pub fn get_city_mut(&mut self, position: &Position) -> Option<&mut City> {
        let position = self
            .cities
            .iter()
            .position(|city| &city.position == position)?;
        Some(&mut self.cities[position])
    }

    pub fn take_city(&mut self, position: &Position) -> Option<City> {
        Some(
            self.cities.remove(
                self.cities
                    .iter()
                    .position(|city| &city.position == position)?,
            ),
        )
    }

    pub fn construct(
        &mut self,
        building: &Building,
        city_position: &Position,
        port_position: Option<Position>,
    ) {
        self.take_events(|events, player| {
            events.on_construct.trigger(player, city_position, building);
        });
        let index = self.index;
        let city = self
            .get_city_mut(city_position)
            .expect("player should be have the this city");
        city.activate();
        city.city_pieces.set_building(building, index);
        if let Some(port_position) = port_position {
            city.port_position = Some(port_position);
        }
        if matches!(building, Academy) {
            self.gain_resources(ResourcePile::ideas(2))
        }
        self.available_buildings -= building;
    }

    pub fn undo_construct(&mut self, building: &Building, city_position: &Position) {
        self.take_events(|events, player| {
            events
                .on_undo_construct
                .trigger(player, city_position, building);
        });
        let city = self
            .get_city_mut(city_position)
            .expect("player should have city");
        city.undo_activate();
        city.city_pieces.remove_building(building);
        if matches!(building, Port) {
            city.port_position = None;
        }
        if matches!(building, Academy) {
            self.loose_resources(ResourcePile::ideas(2))
        }
        self.available_buildings += building;
    }

    fn get_events(&self) -> &PlayerEvents {
        self.events.as_ref().expect("events should be set")
    }

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
    wonders: Vec<String>,
    wonders_build: usize,
    leader_position: Option<Position>,
    game_event_tokens: u8,
    influenced_buildings: u32,
    completed_objectives: Vec<String>,
    defeated_leaders: Vec<String>,
    event_victory_points: f32,
    wonder_cards: Vec<String>,
    available_buildings: AvailableBuildings,
    collect_options: Vec<(Terrain, Vec<ResourcePile>)>,
}
