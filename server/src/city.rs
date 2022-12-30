use crate::{building::Building};

use MoodState::*;

pub struct City {
    pub buildings: Vec<Building>,
    pub mood_state: MoodState,
    pub is_activated: bool,
}

impl City {
    pub fn new() -> Self {
        City {
            buildings: Vec::new(),
            mood_state: Neutral,
            is_activated: false,
        }
    }

    pub fn build_building(&mut self, building: Building) {
        self.buildings.push(building);
    }
}

pub enum MoodState {
    Angry,
    Neutral,
    Happy,
}
