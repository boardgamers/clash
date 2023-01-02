use crate::{
    content::wonders,
    player::Player,
    resource_pile::ResourcePile,
    wonder::{Wonder, WONDER_VICTORY_POINTS},
};

use serde::{Deserialize, Serialize};
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
    pub player: String,
}

impl City {
    pub fn from_data(data: CityData) -> Self {
        Self {
            buildings: Buildings::from_data(data.buildings),
            mood_state: data.mood_state,
            is_activated: data.is_activated,
            player: data.player,
        }
    }

    pub fn to_data(self) -> CityData {
        CityData::new(
            self.buildings.to_data(),
            self.mood_state,
            self.is_activated,
            self.player,
        )
    }

    pub fn new(player: String) -> Self {
        Self {
            buildings: Buildings::default(),
            mood_state: Neutral,
            is_activated: false,
            player,
        }
    }

    pub fn activate(&mut self) {
        if self.is_activated {
            self.decrease_mood_state();
        }
        self.is_activated = true;
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
        player
            .events()
            .city_size_increase_cost
            .trigger(&mut cost, self, &building);
        player.resources().can_afford(&cost)
    }

    pub fn increase_size(&mut self, building: Building, player: &mut Player) {
        let mut events = player.take_events();
        events.city_size_increase.trigger(player, self, &building);
        player.set_events(events);
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
        self.buildings.add_building(building, self.player.clone());
    }

    pub fn conquer(&mut self, new_player: &mut Player, old_player: &mut Player) {
        self.player = new_player.name.clone();
        self.mood_state = Angry;
        if let Some(wonder) = &self.buildings.wonder {
            (wonder.player_deinitializer)(old_player);
            (wonder.player_initializer)(new_player);
            new_player.gain_victory_points(WONDER_VICTORY_POINTS / 2.0 - 1.0);
            old_player.loose_victory_points(WONDER_VICTORY_POINTS / 2.0 - 1.0);
        }
        new_player.gain_victory_points(self.size() as f32);
        old_player.loose_victory_points(self.size() as f32);
        if self.buildings.obelisk.is_some() {
            new_player.loose_victory_points(1.0);
            old_player.gain_victory_points(1.0);
        }
        self.buildings.change_player(new_player.name.clone());
    }

    pub fn size(&self) -> usize {
        self.buildings.amount() + 1
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

#[derive(Serialize, Deserialize)]
pub struct CityData {
    buildings: BuildingsData,
    mood_state: MoodState,
    is_activated: bool,
    player: String,
}

impl CityData {
    pub fn new(
        buildings: BuildingsData,
        mood_state: MoodState,
        is_activated: bool,
        player: String,
    ) -> Self {
        Self {
            buildings,
            mood_state,
            is_activated,
            player,
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
    pub academy: Option<String>,
    pub market: Option<String>,
    pub obelisk: Option<String>,
    pub apothecary: Option<String>,
    pub fortress: Option<String>,
    pub port: Option<String>,
    pub temple: Option<String>,
    pub wonder: Option<Wonder>,
}

impl Buildings {
    fn from_data(data: BuildingsData) -> Self {
        Self::new(
            data.academy,
            data.market,
            data.obelisk,
            data.apothecary,
            data.fortress,
            data.port,
            data.temple,
            data.wonder.map(wonders::get_wonder_by_name),
        )
    }

    fn to_data(self) -> BuildingsData {
        BuildingsData::new(
            self.academy,
            self.market,
            self.obelisk,
            self.apothecary,
            self.fortress,
            self.port,
            self.temple,
            self.wonder.map(|wonder| wonder.name),
        )
    }

    fn new(
        academy: Option<String>,
        market: Option<String>,
        obelisk: Option<String>,
        apothecary: Option<String>,
        fortress: Option<String>,
        port: Option<String>,
        temple: Option<String>,
        wonder: Option<Wonder>,
    ) -> Self {
        Self {
            academy,
            market,
            obelisk,
            apothecary,
            fortress,
            port,
            temple,
            wonder,
        }
    }

    fn can_add_building(&self, building: &Building) -> bool {
        match building {
            Academy => self.academy.is_none(),
            Market => self.market.is_none(),
            Obelisk => self.obelisk.is_none(),
            Apothecary => self.apothecary.is_none(),
            Fortress => self.fortress.is_none(),
            Port => self.port.is_none(),
            Temple => self.temple.is_none(),
            Wonder(_) => self.wonder.is_none(),
        }
    }

    fn add_building(&mut self, building: Building, player: String) {
        match building {
            Academy => self.academy = Some(player),
            Market => self.market = Some(player),
            Obelisk => self.obelisk = Some(player),
            Apothecary => self.apothecary = Some(player),
            Fortress => self.fortress = Some(player),
            Port => self.port = Some(player),
            Temple => self.temple = Some(player),
            Wonder(wonder) => self.wonder = Some(wonder),
        }
    }

    fn amount(&self) -> usize {
        let mut amount = 0;
        if self.academy.is_some() {
            amount += 1;
        }
        if self.market.is_some() {
            amount += 1;
        }
        if self.obelisk.is_some() {
            amount += 1;
        }
        if self.apothecary.is_some() {
            amount += 1;
        }
        if self.fortress.is_some() {
            amount += 1;
        }
        if self.port.is_some() {
            amount += 1;
        }
        if self.temple.is_some() {
            amount += 1;
        }
        if self.wonder.is_some() {
            amount += 1;
        }
        amount
    }

    fn change_player(&mut self, new_player: String) {
        if let Some(academy) = self.academy.as_mut() {
            *academy = new_player.clone();
        }
        if let Some(market) = self.market.as_mut() {
            *market = new_player.clone();
        }
        if let Some(apothecary) = self.apothecary.as_mut() {
            *apothecary = new_player.clone();
        }
        if let Some(fortress) = self.fortress.as_mut() {
            *fortress = new_player.clone();
        }
        if let Some(port) = self.port.as_mut() {
            *port = new_player.clone();
        }
        if let Some(temple) = self.temple.as_mut() {
            *temple = new_player;
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct BuildingsData {
    academy: Option<String>,
    market: Option<String>,
    obelisk: Option<String>,
    apothecary: Option<String>,
    fortress: Option<String>,
    port: Option<String>,
    temple: Option<String>,
    wonder: Option<String>,
}

impl BuildingsData {
    pub fn new(
        academy: Option<String>,
        market: Option<String>,
        obelisk: Option<String>,
        apothecary: Option<String>,
        fortress: Option<String>,
        port: Option<String>,
        temple: Option<String>,
        wonder: Option<String>,
    ) -> Self {
        Self {
            academy,
            market,
            obelisk,
            apothecary,
            fortress,
            port,
            temple,
            wonder,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum MoodState {
    Happy,
    Neutral,
    Angry,
}
