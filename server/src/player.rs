use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering::{self, *},
    collections::{HashMap, VecDeque},
    fmt::Display,
    mem,
};

use crate::{
    advance::Advance,
    army::Unit,
    city::{Building, City, CityData},
    civilization::Civilization,
    content::{advances, civilizations, wonders},
    events::EventMut,
    game::Game,
    hexagon::Position,
    leader::Leader,
    resource_pile::ResourcePile,
    special_advance::SpecialAdvance,
    wonder::Wonder,
};
pub const BUILDING_COST: ResourcePile = ResourcePile {
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
    pub id: usize,
    resources: ResourcePile,
    pub resource_limit: ResourcePile,
    events: Option<PlayerEvents>,
    event_listener_indices: HashMap<String, VecDeque<usize>>,
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
    pub custom_actions: Vec<String>,
    pub wonder_cards: Vec<Wonder>,
}

impl Player {
    pub fn from_data(data: PlayerData) -> Self {
        let mut player = Self {
            name: data.name,
            id: data.id,
            resources: data.resources,
            resource_limit: data.resource_limit,
            events: Some(PlayerEvents::default()),
            event_listener_indices: HashMap::new(),
            cities: data.cities.into_iter().map(City::from_data).collect(),
            units: data.units,
            civilization: civilizations::get_civilization_by_name(&data.civilization)
                .expect("player data should have a valid civilization"),
            active_leader: data
                .active_leader
                .map(|leader| civilizations::get_leader_by_name(&leader, &data.civilization))
                .expect("player data should contain a valid leader"),
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
            game_event_tokens: data.event_tokens,
            influenced_buildings: data.influenced_buildings,
            completed_objectives: data.completed_objectives,
            defeated_leaders: data.defeated_leaders,
            event_victory_points: data.event_victory_points,
            custom_actions: data.custom_actions,
            wonder_cards: data.wonder_cards.iter().map(|wonder| wonders::get_wonder_by_name(wonder).expect("player data should have valid wonder cards")).collect(),
        };
        let advances = mem::take(&mut player.advances);
        for advance in advances.iter() {
            player.advance(advance);
        }
        player.advances = advances;
        if let Some(leader) = player.active_leader.take() {
            (leader.player_initializer)(&mut player);
            player.active_leader = Some(leader);
        }
        let mut cities = mem::take(&mut player.cities);
        for city in cities.iter_mut() {
            for wonder in city.city_pieces.wonders.iter() {
                (wonder.player_initializer)(&mut player);
            }
        }
        player.cities.append(&mut cities);
        player
    }

    pub fn data(self) -> PlayerData {
        PlayerData {
            name: self.name,
            id: self.id,
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
            event_tokens: self.game_event_tokens,
            influenced_buildings: self.influenced_buildings,
            completed_objectives: self.completed_objectives,
            defeated_leaders: self.defeated_leaders,
            event_victory_points: self.event_victory_points,
            custom_actions: self.custom_actions,
            wonder_cards: self.wonder_cards.into_iter().map(|wonder| wonder.name).collect(),
        }
    }

    pub fn new(civilization: Civilization, id: usize) -> Self {
        Self {
            name: None,
            id,
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(2, 7, 7, 7, 7, 7, 7),
            events: Some(PlayerEvents::default()),
            event_listener_indices: HashMap::new(),
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
            custom_actions: Vec::new(),
            wonder_cards: Vec::new(),
        }
    }

    pub fn name(&self) -> String {
        self.name
            .as_ref()
            .expect("name should be set at this point")
            .clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
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

    pub fn kill_leader(&mut self) {
        if let Some(leader) = self.active_leader.take() {
            (leader.player_deinitializer)(self);
        }
    }

    pub fn set_active_leader(&mut self, index: usize) {
        self.kill_leader();
        let new_leader = self.available_leaders.remove(index);
        (new_leader.player_initializer)(self);
        self.active_leader = Some(new_leader);
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
        if self.resources.food + self.resources.ideas + (self.resources.gold as u32) < ADVANCE_COST
        {
            return false;
        }
        self.can_advance_free(advance)
    }

    pub fn has_advance(&self, advance: &String) -> bool {
        self.advances.iter().any(|advances| advances == advance)
    }

    pub fn advance(&mut self, advance: &str) {
        let advance = advances::get_advance_by_name(advance).expect("advance should exist");
        if let Some(advance_bonus) = &advance.advance_bonus {
            self.gain_resources(advance_bonus.resources());
        }
        for i in 0..self.civilization.special_advances.len() {
            if self.civilization.special_advances[i].required_advance == advance.name {
                let special_advance = self.civilization.special_advances.remove(i);
                self.unlock_special_advance(&special_advance);
                self.civilization
                    .special_advances
                    .insert(i, special_advance);
                break;
            }
        }
        (advance.player_initializer)(self);
        self.advances.push(advance.name);
        self.game_event_tokens -= 1;
        if self.game_event_tokens == 0 {
            self.game_event_tokens = 3;
            self.trigger_game_event();
        }
    }

    pub fn remove_advance(&mut self, advance: &Advance) {
        if let Some(position) = self
            .advances
            .iter()
            .position(|advances| advances == &advance.name)
        {
            (advance.player_deinitializer)(self);
            self.advances.remove(position);
        }
    }

    fn unlock_special_advance(&mut self, special_advance: &SpecialAdvance) {
        (special_advance.player_initializer)(self);
        self.unlocked_special_advances
            .push(special_advance.name.clone());
    }

    pub fn victory_points(&self) -> f32 {
        let mut victory_points = 0.0;
        for city in self.cities.iter() {
            victory_points += city.uninfluenced_buildings() as f32 * BUILDING_VICTORY_POINTS;
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

    pub fn events(&self) -> &PlayerEvents {
        self.events
            .as_ref()
            .expect("Events should be set after use")
    }

    pub fn with_events<F>(&mut self, action: F)
    where
        F: FnOnce(&mut Player, &PlayerEvents),
    {
        let events = self.events.take().expect("Events should be set after use");
        action(self, &events);
        self.events = Some(events);
    }

    pub fn conquer_city(&mut self, position: &Position, new_player: &mut Player) {
        self.take_city(&position)
            .expect("player should own city")
            .conquer(new_player, self);
    }

    pub fn with_city<F>(&mut self, position: &Position, action: F)
    where
        F: FnOnce(&mut Player, &mut City),
    {
        let mut city = self
            .take_city(position)
            .expect("player should have this city");
        action(self, &mut city);
        self.cities.push(city);
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

    fn trigger_game_event(&mut self) {
        todo!()
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = Vec::new();
        //todo! strip information about other hand cards
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

    pub fn building_cost(&self, building: &Building, city: &City) -> ResourcePile {
        let mut cost = BUILDING_COST;
        self.events()
            .building_cost
            .trigger(&mut cost, city, building);
        cost
    }

    pub fn wonder_cost(&self, wonder: &Wonder, city: &City) -> ResourcePile {
        let mut cost = wonder.cost.clone();
        self.events().wonder_cost.trigger(&mut cost, city, wonder);
        cost
    }

    pub fn get_city(&mut self, position: &Position) -> Option<&mut City> {
        let position = self
            .cities
            .iter()
            .position(|city| &city.position == position)?;
        Some(&mut self.cities[position])
    }

    fn take_city(&mut self, position: &Position) -> Option<City> {
        Some(
            self.cities.remove(
                self.cities
                    .iter()
                    .position(|city| &city.position == position)?,
            ),
        )
    }

    pub fn raze_city(&mut self, position: &Position, game: &mut Game) {
        let city = self
            .take_city(position)
            .expect("player should have this city");
        city.raze(self, game)
    }
}

#[derive(Serialize, Deserialize)]
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
    event_tokens: u8,
    influenced_buildings: u32,
    completed_objectives: Vec<String>,
    defeated_leaders: Vec<String>,
    event_victory_points: f32,
    custom_actions: Vec<String>,
    wonder_cards: Vec<String>,
}

#[derive(Default)]
pub struct PlayerEvents {
    pub city_size_increase: EventMut<Player, City, Building>,
    pub building_cost: EventMut<ResourcePile, City, Building>,
    pub wonder_cost: EventMut<ResourcePile, City, Wonder>,
}

pub type PlayerInitializer = Box<dyn Fn(&mut Player)>;

pub trait PlayerSetup: Display + Sized {
    fn add_player_initializer(self, initializer: PlayerInitializer) -> Self;
    fn add_player_deinitializer(self, deinitializer: PlayerInitializer) -> Self;

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U, V> + 'static + Clone,
        F: Fn(&mut T, &U, &V) + 'static + Clone,
    {
        let key = self.to_string();
        let deinitialize_event = event.clone();
        let initializer = Box::new(move |player: &mut Player| {
            player
                .event_listener_indices
                .entry(key.clone())
                .or_default()
                .push_back(
                    event(
                        player
                            .events
                            .as_mut()
                            .expect("Events should be set after use"),
                    )
                    .add_listener_mut(listener.clone(), priority),
                )
        });
        let key = self.to_string();
        let deinitializer = Box::new(move |player: &mut Player| {
            deinitialize_event(
                player
                    .events
                    .as_mut()
                    .expect("Events should be set after use"),
            )
            .remove_listener_mut(
                player
                    .event_listener_indices
                    .entry(key.clone())
                    .or_default()
                    .pop_front()
                    .unwrap_or_else(|| panic!("{}: tried to remove non-existing element", key)),
            )
        });
        self.add_player_initializer(initializer)
            .add_player_deinitializer(deinitializer)
    }

    fn add_custom_action(self, action: &str) -> Self {
        let action = action.to_string();
        self.add_player_initializer(Box::new(move |player: &mut Player| {
            player.custom_actions.push(action.clone())
        }))
    }
}

pub fn join_player_initializers(setup: Vec<PlayerInitializer>) -> PlayerInitializer {
    Box::new(move |player: &mut Player| {
        for initializer in setup.iter() {
            initializer(player)
        }
    })
}
