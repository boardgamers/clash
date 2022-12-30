use std::collections::{HashMap, VecDeque};

use crate::{
    building::Building,
    city::City,
    events::EventMut,
    leader::Leader,
    resource_pile::ResourcePile,
    technology::Technology,
    wonder::{Wonder, WONDER_VICTORY_POINTS},
    Civilization,
};

pub struct Player {
    pub name: String,
    pub resources: ResourcePile,
    pub events: PlayerEvents,
    event_listener_indices: HashMap<String, VecDeque<usize>>,
    pub cities: Vec<usize>,
    pub units: Vec<usize>,
    pub civilization: Civilization,
    pub active_leader: Option<Leader>,
    pub available_leaders: Vec<Leader>,
    pub researched_technologies: Vec<usize>,
    pub leader_position: Option<usize>,
    pub victory_points: u32,
}

impl Player {
    pub fn new(name: &str, mut civilization: Civilization) -> Self {
        let mut leaders = Vec::new();
        leaders.append(&mut civilization.leaders);
        Self {
            name: name.to_string(),
            resources: ResourcePile::default(),
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

    pub fn research_technology(&mut self, technology: &Technology) {}

    pub fn build_wonder(&mut self, wonder: Wonder, city: &mut City) {
        (wonder.player_initializer)(self);
        self.victory_points += WONDER_VICTORY_POINTS;
        city.build_building(Building::Wonder(wonder));
    }
}

#[derive(Default)]
pub struct PlayerEvents {
    pub some_event: EventMut<i32, String>,
}

pub type PlayerInitializer = Box<dyn Fn(&mut Player)>;

pub trait PlayerSetup {
    fn add_player_initializer(self, initializer: PlayerInitializer) -> Self;
    fn add_player_deinitializer(self, deinitializer: PlayerInitializer) -> Self;
    fn name(&self) -> String;

    fn add_player_event_listener<T, U, E, F>(self, event: E, listener: F, priority: i32) -> Self
    where
        Self: Sized,
        E: Fn(&mut PlayerEvents) -> &mut EventMut<T, U> + 'static + Clone,
        F: Fn(&mut T, &U) + 'static + Clone,
    {
        let name = self.name();
        let deinitialize_event = event.clone();
        let initializer = Box::new(move |player: &mut Player| {
            player
                .event_listener_indices
                .entry(name.clone())
                .or_default()
                .push_back(event(&mut player.events).add_listener_mut(listener.clone(), priority))
        });
        let name = self.name();
        let deinitializer = Box::new(move |player: &mut Player| {
            deinitialize_event(&mut player.events).remove_listener_mut(
                player
                    .event_listener_indices
                    .entry(name.clone())
                    .or_default()
                    .pop_front()
                    .unwrap_or_else(|| {
                        panic!("{}: tried to remove non-existing element", name)
                    }),
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
