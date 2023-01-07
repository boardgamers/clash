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

    pub fn can_increase_size(&self, city_piece: &CityPiece, player: &mut Player) -> bool {
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if matches!(self.mood_state, Angry) {
            return false;
        }
        if self.city_pieces.can_add_city_piece(city_piece) {
            return false;
        }
        let mut cost = match &city_piece {
            CityPiece::Wonder(wonder) => wonder.cost.clone(),
            _ => BUILDING_COST,
        };
        player
            .events()
            .city_size_increase_cost
            .trigger(&mut cost, self, city_piece);
        player.resources().can_afford(&cost)
    }

    pub fn increase_size(&mut self, city_piece: CityPiece, player: &mut Player) {
        self.activate();
        let mut events = player.take_events();
        events.city_size_increase.trigger(player, self, &city_piece);
        player.set_events(events);
        match &city_piece {
            CityPiece::Academy => player.gain_resources(ResourcePile::ideas(2)),
            CityPiece::Wonder(wonder) => {
                (wonder.player_initializer)(player);
                player.wonders.push(wonder.name.clone());
                player.wonders_build += 1;
            }
            _ => (),
        }
        self.city_pieces
            .add_city_piece(city_piece, self.player.clone());
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

    pub fn buildings(&self) -> u32 {
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

    pub fn influence_culture(&mut self, player: &mut Player, building: CityPiece) {
        todo!()
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

pub enum CityPiece {
    Academy,
    Market,
    Obelisk,
    Observatory,
    Fortress,
    Port,
    Temple,
    Wonder(Wonder),
}

impl CityPiece {
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

    fn to_data(&self) -> CityPieceData {
        match self {
            Self::Academy => CityPieceData::Academy,
            Self::Market => CityPieceData::Market,
            Self::Obelisk => CityPieceData::Obelisk,
            Self::Observatory => CityPieceData::Observatory,
            Self::Fortress => CityPieceData::Fortress,
            Self::Port => CityPieceData::Port,
            Self::Temple => CityPieceData::Temple,
            Self::Wonder(wonder) => CityPieceData::Wonder(wonder.name.clone()),
        }
    }

    pub fn from_data(data: &CityPieceData) -> Self {
        match data {
            CityPieceData::Academy => Self::Academy,
            CityPieceData::Market => Self::Market,
            CityPieceData::Obelisk => Self::Obelisk,
            CityPieceData::Observatory => Self::Observatory,
            CityPieceData::Fortress => Self::Fortress,
            CityPieceData::Port => Self::Port,
            CityPieceData::Temple => Self::Temple,
            CityPieceData::Wonder(name) => Self::Wonder(
                wonders::get_wonder_by_name(name)
                    .expect("city piece data should have a valid wonder name"),
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum CityPieceData {
    Academy,
    Market,
    Obelisk,
    Observatory,
    Fortress,
    Port,
    Temple,
    Wonder(String),
}

impl Display for CityPieceData {
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
                Self::Wonder(name) => return write!(f, "the wonder \"{name}\""),
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
                    wonders::get_wonder_by_name(&wonder)
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

    fn can_add_city_piece(&self, city_piece: &CityPiece) -> bool {
        match city_piece {
            CityPiece::Academy => self.academy.is_none(),
            CityPiece::Market => self.market.is_none(),
            CityPiece::Obelisk => self.obelisk.is_none(),
            CityPiece::Observatory => self.observatory.is_none(),
            CityPiece::Fortress => self.fortress.is_none(),
            CityPiece::Port => self.port.is_none(),
            CityPiece::Temple => self.temple.is_none(),
            CityPiece::Wonder(_) => true,
        }
    }

    fn add_city_piece(&mut self, city_piece: CityPiece, player: String) {
        match city_piece {
            CityPiece::Academy => self.academy = Some(player),
            CityPiece::Market => self.market = Some(player),
            CityPiece::Obelisk => self.obelisk = Some(player),
            CityPiece::Observatory => self.observatory = Some(player),
            CityPiece::Fortress => self.fortress = Some(player),
            CityPiece::Port => self.port = Some(player),
            CityPiece::Temple => self.temple = Some(player),
            CityPiece::Wonder(wonder) => self.wonders.push(wonder),
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
