use std::{collections::{HashMap, VecDeque}, fmt::Display};

use crate::{
    army::Unit, city::{City, Building}, events::EventMut, hexagon::HexagonPosition, leader::Leader,
    resource_pile::ResourcePile, special_technology::SpecialTechnology, technology::Technology,
    Civilization,
};

const TECHNOLOGY_COST: u32 = 2;

pub struct Player {
    pub name: String,
    pub resources: ResourcePile,
    pub resource_limit: ResourcePile,
    pub events: PlayerEvents,
    event_listener_indices: HashMap<String, VecDeque<usize>>,
    pub cities: Vec<City>,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub researched_technologies: Vec<String>,
    pub leader_position: Option<HexagonPosition>,
    victory_points: u32,
}

impl Player {
    pub fn new(name: &str, mut civilization: Civilization) -> Self {
        let mut leaders = Vec::new();
        leaders.append(&mut civilization.leaders);
        Self {
            name: name.to_string(),
            resources: ResourcePile::food(2),
            resource_limit: ResourcePile::new(7, 7, 7, 2, 7, 7, 7),
            events: PlayerEvents::default(),
            event_listener_indices: HashMap::new(),
            cities: Vec::new(),
            units: Vec::new(),
            civilization,
            active_leader: None,
            available_leaders: leaders,
            researched_technologies: Vec::new(),
            leader_position: None,
            victory_points: 0,
        }
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.resources += resources;
        self.resources.apply_resource_limit(&self.resource_limit);
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
                self.unlock_special_technology(special_technology);
                break;
            }
        }
        (technology.player_initializer)(self);
        self.researched_technologies.push(technology.name.clone());
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

    fn unlock_special_technology(&mut self, special_technology: SpecialTechnology) {
        (special_technology.player_initializer)(self);
    }

    pub fn victory_points(&self) -> u32 {
        self.victory_points / 2
    }

    pub fn gain_victory_points(&mut self, victory_points: f32) {
        self.victory_points += (victory_points * 2.0) as u32;
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
                .push_back(event(&mut player.events).add_listener_mut(listener.clone(), priority))
        });
        let key = self.to_string();
        let deinitializer = Box::new(move |player: &mut Player| {
            deinitialize_event(&mut player.events).remove_listener_mut(
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
