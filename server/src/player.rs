use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
};

use crate::{
    army::Unit,
    city::{Building, City, CityData},
    civilization::Civilization,
    content::{civilizations, technologies},
    events::EventMut,
    hexagon::HexagonPosition,
    leader::Leader,
    resource_pile::ResourcePile,
    special_technology::SpecialTechnology,
    technology::Technology,
};

const TECHNOLOGY_COST: u32 = 2;

pub struct Player {
    pub name: String,
    resources: ResourcePile,
    pub resource_limit: ResourcePile,
    events: Option<PlayerEvents>,
    event_listener_indices: HashMap<String, VecDeque<usize>>,
    pub cities: Vec<City>,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub researched_technologies: Vec<String>,
    pub leader_position: Option<HexagonPosition>,
    event_tokens: u8,
    victory_points: u32,
}

impl Player {
    pub fn from_data(data: PlayerData) -> Self {
        let mut player = Self {
            name: data.name,
            resources: data.resources,
            resource_limit: data.resource_limit,
            events: Some(PlayerEvents::default()),
            event_listener_indices: HashMap::new(),
            cities: data.cities.into_iter().map(City::from_data).collect(),
            units: data.units,
            civilization: civilizations::get_civilization_by_name(data.civilization.clone()),
            active_leader: data
                .active_leader
                .map(|leader| civilizations::get_leader_by_name(leader, data.civilization.clone())),
            available_leaders: data
                .available_leaders
                .into_iter()
                .map(|leader| civilizations::get_leader_by_name(leader, data.civilization.clone()))
                .collect(),
            researched_technologies: data.researched_technologies,
            leader_position: data.leader_position,
            event_tokens: data.event_tokens,
            victory_points: data.victory_points,
        };
        let technologies = player
            .researched_technologies
            .iter()
            .map(|technology| technologies::get_technology_by_name(technology.clone()))
            .collect::<Vec<Technology>>();
        for technology in technologies.into_iter() {
            player.research_technology(&technology);
        }
        if let Some(leader) = player.active_leader.take() {
            (leader.player_initializer)(&mut player);
            player.active_leader = Some(leader);
        }
        let mut cities = Vec::new();
        cities.append(&mut player.cities);
        for city in cities.iter_mut() {
            if let Some(wonder) = city.buildings.wonder.take() {
                (wonder.player_initializer)(&mut player);
                city.buildings.wonder = Some(wonder);
            }
        }
        player.cities.append(&mut cities);
        player
    }

    pub fn to_data(self) -> PlayerData {
        PlayerData::new(
            self.name,
            self.resources,
            self.resource_limit,
            self.cities.into_iter().map(|city| city.to_data()).collect(),
            self.units,
            self.civilization.name,
            self.active_leader.map(|leader| leader.name),
            self.available_leaders
                .into_iter()
                .map(|leader| leader.name)
                .collect(),
            self.researched_technologies,
            self.leader_position,
            self.event_tokens,
            self.victory_points,
        )
    }

    pub fn new(name: &str, mut civilization: Civilization) -> Self {
        let mut leaders = Vec::new();
        leaders.append(&mut civilization.leaders);
        Self {
            name: name.to_string(),
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(2, 7, 7, 7, 7, 7, 7),
            events: Some(PlayerEvents::default()),
            event_listener_indices: HashMap::new(),
            cities: Vec::new(),
            units: Vec::new(),
            civilization,
            active_leader: None,
            available_leaders: leaders,
            researched_technologies: Vec::new(),
            leader_position: None,
            event_tokens: 3,
            victory_points: 0,
        }
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

    pub fn can_research_technology(&self, technology: &Technology) -> bool {
        if self.resources.food + self.resources.ideas + self.resources.gold < TECHNOLOGY_COST {
            return false;
        }
        if self.has_technology(&technology.name) {
            return false;
        }
        if let Some(required_technology) = &technology.required_technology {
            if !self.has_technology(required_technology) {
                return false;
            }
        }
        if let Some(contradicting_technology) = &technology.contradicting_technology {
            if self.has_technology(contradicting_technology) {
                return false;
            }
        }
        true
    }

    pub fn has_technology(&self, technology: &String) -> bool {
        self.researched_technologies
            .iter()
            .any(|research_technology| research_technology == technology)
    }

    pub fn research_technology(&mut self, technology: &Technology) {
        if let Some(advance_bonus) = &technology.advance_bonus {
            self.gain_resources(advance_bonus.resources());
        }
        for i in 0..self.civilization.special_technologies.len() {
            if self.civilization.special_technologies[i].required_technology == technology.name {
                let special_technology = self.civilization.special_technologies.remove(i);
                self.unlock_special_technology(&special_technology);
                self.civilization
                    .special_technologies
                    .insert(i, special_technology);
                break;
            }
        }
        (technology.player_initializer)(self);
        self.researched_technologies.push(technology.name.clone());
        self.event_tokens -= 1;
        if self.event_tokens == 0 {
            self.event_tokens = 3;
            self.trigger_game_event();
        }
    }

    pub fn remove_technology(&mut self, technology: &Technology) {
        if let Some(position) = self
            .researched_technologies
            .iter()
            .position(|researched_technology| researched_technology == &technology.name)
        {
            (technology.player_deinitializer)(self);
            self.researched_technologies.remove(position);
        }
    }

    fn unlock_special_technology(&mut self, special_technology: &SpecialTechnology) {
        (special_technology.player_initializer)(self);
    }

    pub fn victory_points(&self) -> u32 {
        self.victory_points / 2
    }

    pub fn gain_victory_points(&mut self, victory_points: f32) {
        self.victory_points += (victory_points * 2.0) as u32;
    }

    pub fn loose_victory_points(&mut self, victory_points: f32) {
        self.victory_points -= (victory_points * 2.0) as u32;
    }

    pub fn events(&mut self) -> &mut PlayerEvents {
        self.events
            .as_mut()
            .expect("Events should be set after use")
    }

    pub fn take_events(&mut self) -> PlayerEvents {
        self.events.take().expect("Events should be set after use")
    }

    pub fn set_events(&mut self, events: PlayerEvents) {
        self.events = Some(events);
    }

    pub fn event_tokens(&self) -> u8 {
        self.event_tokens
    }

    fn trigger_game_event(&mut self) {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
pub struct PlayerData {
    name: String,
    resources: ResourcePile,
    resource_limit: ResourcePile,
    cities: Vec<CityData>,
    units: Vec<Unit>,
    civilization: String,
    active_leader: Option<String>,
    available_leaders: Vec<String>,
    researched_technologies: Vec<String>,
    leader_position: Option<HexagonPosition>,
    event_tokens: u8,
    victory_points: u32,
}

impl PlayerData {
    pub fn new(
        name: String,
        resources: ResourcePile,
        resource_limit: ResourcePile,
        cities: Vec<CityData>,
        units: Vec<Unit>,
        civilization: String,
        active_leader: Option<String>,
        available_leaders: Vec<String>,
        researched_technologies: Vec<String>,
        leader_position: Option<HexagonPosition>,
        event_tokens: u8,
        victory_points: u32,
    ) -> Self {
        Self {
            name,
            resources,
            resource_limit,
            cities,
            units,
            civilization,
            active_leader,
            available_leaders,
            researched_technologies,
            leader_position,
            event_tokens,
            victory_points,
        }
    }
}

#[derive(Default)]
pub struct PlayerEvents {
    pub some_event: EventMut<i32, String>,
    pub city_size_increase: EventMut<Player, City, Building>,
    pub city_size_increase_cost: EventMut<ResourcePile, City, Building>,
}

pub type PlayerInitializer = Box<dyn Fn(&mut Player)>;

pub trait PlayerSetup: Display {
    fn add_player_initializer(self, initializer: PlayerInitializer) -> Self;
    fn add_player_deinitializer(self, deinitializer: PlayerInitializer) -> Self;

    fn add_player_event_listener<T, U, V, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        Self: Sized,
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
}

pub fn join_player_initializers(setup: Vec<PlayerInitializer>) -> PlayerInitializer {
    Box::new(move |player: &mut Player| {
        for initializer in setup.iter() {
            initializer(player)
        }
    })
}
