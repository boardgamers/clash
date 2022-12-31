use crate::{
    player::Player,
    resource_pile::ResourcePile,
    wonder::{Wonder, WONDER_VICTORY_POINTS},
};

use Building::*;
use MoodState::*;

const MAX_CITY_SIZE: usize = 4;
const CITY_PIECE_VICTORY_POINTS: f32 = 1.0;
const BUILDING_COST: ResourcePile = ResourcePile {
    wood: 1,
    stone: 1,
    gold: 0,
    food: 1,
    ideas: 0,
    mood_tokens: 0,
    culture_tokens: 0,
};

pub struct City {
    pub buildings: Buildings,
    pub mood_state: MoodState,
    pub is_activated: bool,
}

impl City {
    pub fn new() -> Self {
        Self {
            buildings: Buildings::default(),
            mood_state: Neutral,
            is_activated: false,
        }
    }

    pub fn can_increase_size(&self, building: Building, player: &mut Player) -> bool {
        if self.buildings.amount() == MAX_CITY_SIZE {
            return false;
        }
        if self.buildings.can_add_building(&building) {
            return false;
        }
        let mut cost = match &building {
            Wonder(wonder) => wonder.cost.clone(),
            _ => BUILDING_COST,
        };
        player.events.city_size_increase_cost.trigger(&mut cost, self, &building);
        player.resources.can_afford(&cost)
    }

    pub fn increase_size(&mut self, building: Building, player: &mut Player) {
        player.events.city_size_increase.trigger(player, self, &building);
        let victory_points = match &building {
            Wonder(_) => WONDER_VICTORY_POINTS,
            _ => CITY_PIECE_VICTORY_POINTS,
        };
        player.gain_victory_points(victory_points);
        match &building {
            Academy => player.gain_resources(ResourcePile::ideas(2)),
            Wonder(wonder) => (wonder.player_initializer)(player),
            _ => (),
        }
        self.buildings.add_building(building);
    }

    pub fn size(&self) -> usize {
        self.buildings.amount()
    }

    pub fn mood_modified_size(&self) -> usize {
        match self.mood_state {
            Happy => self.size() + 1,
            Neutral => self.size(),
            Angry => 1,
        }
    }

    pub fn increase_mood_state(&mut self) {
        self.mood_state = match self.mood_state {
            Happy | Neutral => Happy,
            Angry => Neutral,
        }
    }

    pub fn decrease_mood_state(&mut self) {
        self.mood_state = match self.mood_state {
            Happy => Neutral,
            Neutral | Angry => Angry,
        }
    }
}

pub enum Building {
    Academy,
    Market,
    Obelisk,
    Apothecary,
    Fortress,
    Port,
    Temple,
    Wonder(Wonder),
}

#[derive(Default)]
pub struct Buildings {
    academy: bool,
    market: bool,
    obelisk: bool,
    apothecary: bool,
    fortress: bool,
    port: bool,
    temple: bool,
    wonders: Option<Wonder>,
}

impl Buildings {
    fn can_add_building(&self, building: &Building) -> bool {
        match building {
            Academy => !self.academy,
            Market => !self.market,
            Obelisk => !self.obelisk,
            Apothecary => !self.apothecary,
            Fortress => !self.fortress,
            Port => !self.port,
            Temple => !self.temple,
            Wonder(_) => self.wonders.is_none(),
        }
    }

    fn add_building(&mut self, building: Building) {
        match building {
            Academy => self.academy = true,
            Market => self.market = true,
            Obelisk => self.obelisk = true,
            Apothecary => self.apothecary = true,
            Fortress => self.fortress = true,
            Port => self.port = true,
            Temple => self.temple = true,
            Wonder(wonder) => self.wonders = Some(wonder),
        }
    }

    fn amount(&self) -> usize {
        let mut amount = 0;
        if self.academy {
            amount += 1;
        }
        if self.market {
            amount += 1;
        }
        if self.obelisk {
            amount += 1;
        }
        if self.apothecary {
            amount += 1;
        }
        if self.fortress {
            amount += 1;
        }
        if self.port {
            amount += 1;
        }
        if self.temple {
            amount += 1;
        }
        if self.wonders.is_some() {
            amount += 1;
        }
        amount
    }
}

pub enum MoodState {
    Happy,
    Neutral,
    Angry,
}
