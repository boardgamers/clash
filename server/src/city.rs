use std::fmt::Display;

use crate::{
    content::wonders, game::Game, hexagon::Position, player, player::Player,
    resource_pile::ResourcePile, wonder::Wonder,
};

use serde::{Deserialize, Serialize};
use MoodState::*;

const MAX_CITY_SIZE: u32 = 4;

pub struct City {
    pub city_pieces: CityPieces,
    pub mood_state: MoodState,
    pub is_activated: bool,
    pub player: String,
    pub position: Position,
}

impl City {
    pub fn from_data(data: CityData) -> Self {
        Self {
            city_pieces: CityPieces::from_data(data.city_pieces),
            mood_state: data.mood_state,
            is_activated: data.is_activated,
            player: data.player,
            position: data.position,
        }
    }

    pub fn data(self) -> CityData {
        CityData::new(
            self.city_pieces.data(),
            self.mood_state,
            self.is_activated,
            self.player,
            self.position,
        )
    }

    pub fn new(player: String, position: Position) -> Self {
        Self {
            city_pieces: CityPieces::default(),
            mood_state: Neutral,
            is_activated: false,
            player,
            position,
        }
    }

    pub fn activate(&mut self) {
        if self.is_activated {
            self.decrease_mood_state();
        }
        self.is_activated = true;
    }

    pub fn can_increase_size(&self, building: &Building, player: &Player) -> bool {
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if matches!(self.mood_state, Angry) {
            return false;
        }
        if !self.city_pieces.can_add_building(building) {
            return false;
        }
        let cost = player.building_cost(building, self);
        player.resources().can_afford(&cost)
    }

    pub fn increase_size(&mut self, building: &Building, player: &mut Player) {
        self.activate();
        player.with_events(|p, e| e.city_size_increase.trigger(p, self, building));
        if matches!(building, Building::Academy) {
            player.gain_resources(ResourcePile::ideas(2))
        }
        self.city_pieces.set_building(building, self.player.clone());
    }

    pub fn can_build_wonder(&self, wonder: &Wonder, player: &Player) -> bool {
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if !matches!(self.mood_state, Happy) {
            return false;
        }
        let cost = player.wonder_cost(wonder, self);
        if !player.resources().can_afford(&cost) {
            return false;
        }
        for advance in wonder.required_advances.iter() {
            if !player.has_advance(advance) {
                return false;
            }
        }
        //todo! use wonder's placement checker
        true
    }

    pub fn build_wonder(&mut self, wonder: Wonder, player: &mut Player) {
        let mut wonder = wonder;
        (wonder.player_initializer)(player);
        player.wonders_build += 1;
        player.wonders.push(wonder.name.clone());
        wonder.builder = Some(player.name());
        self.city_pieces.wonders.push(wonder);
    }

    pub fn conquer(mut self, new_player: &mut Player, old_player: &mut Player) {
        let new_player_name = new_player.name();
        self.player = new_player_name.clone();
        self.mood_state = Angry;
        for wonder in self.city_pieces.wonders.iter() {
            (wonder.player_deinitializer)(old_player);
            (wonder.player_initializer)(new_player);
            old_player.remove_wonder(wonder);
            new_player.wonders.push(wonder.name.clone());
        }
        if let Some(player) = &self.city_pieces.obelisk {
            if player == &old_player.name() {
                old_player.influenced_buildings += 1;
            }
        }
        let previously_influenced_building =
            self.city_pieces.buildings(Some(&new_player.name())).len() as u32;
        new_player.influenced_buildings -= previously_influenced_building;
        self.city_pieces.change_player(new_player_name);
        new_player.cities.push(self)
    }

    pub fn raze(self, player: &mut Player, game: &mut Game) {
        for wonder in self.city_pieces.wonders.into_iter() {
            (wonder.player_deinitializer)(player);
            player.remove_wonder(&wonder);
            let builder = wonder.builder.expect("Wonder should have a builder");
            let builder = game
                .players
                .iter_mut()
                .find(|player| player.name() == builder)
                .expect("builder should be a player");
            builder.wonders_build -= 1;
        }
    }

    pub fn size(&self) -> u32 {
        self.city_pieces.amount() + 1
    }

    pub fn mood_modified_size(&self) -> u32 {
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

    pub fn uninfluenced_buildings(&self) -> u32 {
        self.city_pieces.buildings(Some(&self.player)).len() as u32
    }

    //this function assumes action is legal
    pub fn influence_culture(&mut self, influencer: &mut Player, building: &Building) {
        self.city_pieces.set_building(building, influencer.name());
        if matches!(building, Building::Obelisk) {
            panic!("obelisks cannot be culturally influenced")
        }
        influencer.influenced_buildings += 1;
    }
}

#[derive(Serialize, Deserialize)]
pub struct CityData {
    city_pieces: CityPiecesData,
    mood_state: MoodState,
    is_activated: bool,
    player: String,
    position: Position,
}

impl CityData {
    pub fn new(
        city_pieces: CityPiecesData,
        mood_state: MoodState,
        is_activated: bool,
        player: String,
        position: Position,
    ) -> Self {
        Self {
            city_pieces,
            mood_state,
            is_activated,
            player,
            position,
        }
    }
}

pub enum Building {
    Academy,
    Market,
    Obelisk,
    Observatory,
    Fortress,
    Port,
    Temple,
}

impl Building {
    pub fn json(&self) -> String {
        serde_json::to_string(&self.to_data()).expect("city piece data should be valid json")
    }

    pub fn from_json(json: &str) -> Self {
        Self::from_data(
            serde_json::from_str(json)
                .as_ref()
                .expect("API call should receive valid city piece data json"),
        )
    }

    fn to_data(&self) -> BuildingData {
        match self {
            Self::Academy => BuildingData::Academy,
            Self::Market => BuildingData::Market,
            Self::Obelisk => BuildingData::Obelisk,
            Self::Observatory => BuildingData::Observatory,
            Self::Fortress => BuildingData::Fortress,
            Self::Port => BuildingData::Port,
            Self::Temple => BuildingData::Temple,
        }
    }

    pub fn from_data(data: &BuildingData) -> Self {
        match data {
            BuildingData::Academy => Self::Academy,
            BuildingData::Market => Self::Market,
            BuildingData::Obelisk => Self::Obelisk,
            BuildingData::Observatory => Self::Observatory,
            BuildingData::Fortress => Self::Fortress,
            BuildingData::Port => Self::Port,
            BuildingData::Temple => Self::Temple,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum BuildingData {
    Academy,
    Market,
    Obelisk,
    Observatory,
    Fortress,
    Port,
    Temple,
}

impl Display for BuildingData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Academy => "an academy",
                Self::Market => "a market",
                Self::Obelisk => "an obelisk",
                Self::Observatory => "an observatory",
                Self::Fortress => "a fortress",
                Self::Port => "a port",
                Self::Temple => "a temple",
            }
        )
    }
}

#[derive(Default)]
pub struct CityPieces {
    pub academy: Option<String>,
    pub market: Option<String>,
    pub obelisk: Option<String>,
    pub observatory: Option<String>,
    pub fortress: Option<String>,
    pub port: Option<String>,
    pub temple: Option<String>,
    pub wonders: Vec<Wonder>,
}

impl CityPieces {
    fn from_data(data: CityPiecesData) -> Self {
        Self {
            academy: data.academy,
            market: data.market,
            obelisk: data.obelisk,
            observatory: data.observatory,
            fortress: data.fortress,
            port: data.port,
            temple: data.temple,
            wonders: data
                .wonders
                .iter()
                .map(|wonder| {
                    wonders::get_wonder_by_name(wonder)
                        .expect("city piece data should contain a valid wonder")
                })
                .collect(),
        }
    }

    fn data(self) -> CityPiecesData {
        CityPiecesData {
            academy: self.academy,
            market: self.market,
            obelisk: self.obelisk,
            observatory: self.observatory,
            fortress: self.fortress,
            port: self.port,
            temple: self.temple,
            wonders: self.wonders.into_iter().map(|wonder| wonder.name).collect(),
        }
    }

    fn can_add_building(&self, building: &Building) -> bool {
        match building {
            Building::Academy => self.academy.is_none(),
            Building::Market => self.market.is_none(),
            Building::Obelisk => self.obelisk.is_none(),
            Building::Observatory => self.observatory.is_none(),
            Building::Fortress => self.fortress.is_none(),
            Building::Port => self.port.is_none(),
            Building::Temple => self.temple.is_none(),
        }
    }

    fn set_building(&mut self, building: &Building, player: String) {
        match building {
            Building::Academy => self.academy = Some(player),
            Building::Market => self.market = Some(player),
            Building::Obelisk => self.obelisk = Some(player),
            Building::Observatory => self.observatory = Some(player),
            Building::Fortress => self.fortress = Some(player),
            Building::Port => self.port = Some(player),
            Building::Temple => self.temple = Some(player),
        }
    }

    fn amount(&self) -> u32 {
        (self.buildings(None).len() + self.wonders.len()) as u32
    }

    fn change_player(&mut self, new_player: String) {
        for b in self.buildings(None) {
            if !matches!(b, Building::Obelisk) {
                self.set_building(&b, new_player.clone());
            }
        }
    }

    pub fn buildings(&self, owned_by: Option<&str>) -> Vec<Building> {
        vec![
            (Building::Academy, self.academy.clone()),
            (Building::Market, self.market.clone()),
            (Building::Obelisk, self.obelisk.clone()),
            (Building::Observatory, self.observatory.clone()),
            (Building::Fortress, self.fortress.clone()),
            (Building::Port, self.port.clone()),
            (Building::Temple, self.temple.clone()),
        ]
        .into_iter()
        .filter_map(|(building, owner)| {
            owner
                .filter(|owner| match owned_by {
                    Some(want_owner) => owner == want_owner,
                    None => true,
                })
                .map(|_| building)
        })
        .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct CityPiecesData {
    academy: Option<String>,
    market: Option<String>,
    obelisk: Option<String>,
    observatory: Option<String>,
    fortress: Option<String>,
    port: Option<String>,
    temple: Option<String>,
    wonders: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum MoodState {
    Happy,
    Neutral,
    Angry,
}
