use std::fmt::Display;

use crate::{
    content::wonders, hexagon::HexagonPosition, player::Player, resource_pile::ResourcePile,
    wonder::Wonder,
};

use serde::{Deserialize, Serialize};
use MoodState::*;

const MAX_CITY_SIZE: u32 = 4;
pub const BUILDING_COST: ResourcePile = ResourcePile {
    food: 1,
    wood: 1,
    ore: 1,
    ideas: 0,
    gold: 0,
    mood_tokens: 0,
    culture_tokens: 0,
};

pub struct City {
    pub city_pieces: CityPieces,
    pub mood_state: MoodState,
    pub is_activated: bool,
    pub player: String,
    pub position: HexagonPosition,
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

    pub fn new(player: String, position: HexagonPosition) -> Self {
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

    pub fn can_increase_size(&self, building: &Building, player: &mut Player) -> bool {
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if matches!(self.mood_state, Angry) {
            return false;
        }
        if self.city_pieces.can_add_building(building) {
            return false;
        }
        let mut cost = BUILDING_COST;
        player
            .events()
            .city_size_increase_cost
            .trigger(&mut cost, self, building);
        player.resources().can_afford(&cost)
    }

    pub fn increase_size(&mut self, building: Building, player: &mut Player) {
        self.activate();
        let mut events = player.take_events();
        events.city_size_increase.trigger(player, self, &building);
        player.set_events(events);
        if let Building::Academy = &building {
            player.gain_resources(ResourcePile::ideas(2))
        }
        self.city_pieces.add_building(building, self.player.clone());
    }

    pub fn conquer(&mut self, new_player: &mut Player, old_player: &mut Player) {
        let new_player_name = new_player.name();
        self.player = new_player_name.clone();
        self.mood_state = Angry;
        for wonder in self.city_pieces.wonders.iter() {
            (wonder.player_deinitializer)(old_player);
            (wonder.player_initializer)(new_player);
            let wonder = old_player.wonders.remove(
                old_player
                    .wonders
                    .iter()
                    .position(|player_wonder| player_wonder == &wonder.name)
                    .expect("player should have conquered wonder"),
            );
            new_player.wonders.push(wonder);
        }
        if let Some(player) = &self.city_pieces.obelisk {
            if player == &old_player.name() {
                old_player.influenced_buildings += 1;
            }
        }
        let mut previously_influenced_building = 0;
        if self.city_pieces.academy == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.market == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.obelisk == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.observatory == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.fortress == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.port == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        if self.city_pieces.temple == Some(new_player_name.clone()) {
            previously_influenced_building += 1;
        }
        new_player.influenced_buildings -= previously_influenced_building;
        self.city_pieces.change_player(new_player_name);
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
        let mut value = 0;
        if self.city_pieces.academy == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.market == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.obelisk == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.observatory == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.fortress == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.port == Some(self.player.clone()) {
            value += 1;
        }
        if self.city_pieces.temple == Some(self.player.clone()) {
            value += 1;
        }
        value
    }

    //this function assumes action is legal
    pub fn influence_culture(&mut self, influencer: &mut Player, building: &Building) {
        match building {
            Building::Academy => self.city_pieces.academy = Some(influencer.name()),
            Building::Market => self.city_pieces.market = Some(influencer.name()),
            Building::Observatory => self.city_pieces.observatory = Some(influencer.name()),
            Building::Fortress => self.city_pieces.fortress = Some(influencer.name()),
            Building::Port => self.city_pieces.port = Some(influencer.name()),
            Building::Temple => self.city_pieces.temple = Some(influencer.name()),
            Building::Obelisk => unreachable!("obelisks cannot be culturally influenced"),
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
    position: HexagonPosition,
}

impl CityData {
    pub fn new(
        city_pieces: CityPiecesData,
        mood_state: MoodState,
        is_activated: bool,
        player: String,
        position: HexagonPosition,
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
                Self::Academy => "a academy",
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

    fn add_building(&mut self, building: Building, player: String) {
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
        if self.observatory.is_some() {
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
        amount += self.wonders.len() as u32;
        amount
    }

    fn change_player(&mut self, new_player: String) {
        if let Some(academy) = self.academy.as_mut() {
            *academy = new_player.clone();
        }
        if let Some(market) = self.market.as_mut() {
            *market = new_player.clone();
        }
        if let Some(observatory) = self.observatory.as_mut() {
            *observatory = new_player.clone();
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

#[derive(Serialize, Deserialize)]
pub enum MoodState {
    Happy,
    Neutral,
    Angry,
}
