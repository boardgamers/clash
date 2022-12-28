use std::collections::HashMap;

use crate::{events::EventMut, leader::Leader, resource_pile::ResourcePile, Civilization};

pub struct Player {
    pub name: String,
    pub resources: ResourcePile,
    pub events: PlayerEvents,
    pub leader_event_listener_indices: Vec<usize>,
    pub wonder_event_listener_indices: HashMap<String, Vec<usize>>,
    pub cities: Vec<usize>,
    pub units: Vec<usize>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub researched_technologies: Vec<usize>,
    pub leader_position: Option<usize>,
}

impl Player {
    pub fn new(name: &str, mut civilization: Civilization) -> Self {
        let mut leaders = Vec::new();
        leaders.append(&mut civilization.leaders);
        Self {
            name: name.to_string(),
            resources: ResourcePile::empty(),
            events: PlayerEvents::default(),
            leader_event_listener_indices: Vec::new(),
            wonder_event_listener_indices: HashMap::new(),
            cities: Vec::new(),
            units: Vec::new(),
            civilization,
            active_leader: None,
            available_leaders: leaders,
            researched_technologies: Vec::new(),
            leader_position: None,
        }
    }

    pub fn kill_leader(&mut self) {
        if let Some(leader) = self.active_leader.take() {
            (leader.deinitializer)(self);
        }
    }

    pub fn set_active_leader(&mut self, index: usize) {
        self.kill_leader();
        let new_leader = self.available_leaders.remove(index);
        (new_leader.initializer)(self);
        self.active_leader = Some(new_leader);
    }
}

#[derive(Default)]
pub struct PlayerEvents {
    pub some_event: EventMut<i32>,
}
