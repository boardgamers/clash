use crate::{content::wonders, game::Game, hexagon::Position, player::Player, wonder::Wonder};

use serde::{Deserialize, Serialize};
use MoodState::*;

use Building::*;

const MAX_CITY_SIZE: usize = 4;

pub struct City {
    pub city_pieces: CityPieces,
    pub mood_state: MoodState,
    pub is_activated: bool,
    pub player_index: usize,
    pub position: Position,
}

impl City {
    pub fn from_data(data: CityData) -> Self {
        Self {
            city_pieces: CityPieces::from_data(data.city_pieces),
            mood_state: data.mood_state,
            is_activated: data.is_activated,
            player_index: data.player_index,
            position: data.position,
        }
    }

    pub fn data(self) -> CityData {
        CityData::new(
            self.city_pieces.data(),
            self.mood_state,
            self.is_activated,
            self.player_index,
            self.position,
        )
    }

    pub fn new(player_index: usize, position: Position) -> Self {
        Self {
            city_pieces: CityPieces::default(),
            mood_state: Neutral,
            is_activated: false,
            player_index,
            position,
        }
    }

    pub fn activate(&mut self) {
        if self.is_activated {
            self.decrease_mood_state();
        }
        self.is_activated = true;
    }

    pub fn can_construct(&self, building: &Building, player: &Player) -> bool {
        if self.player_index != player.index {
            return false;
        }
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if matches!(self.mood_state, Angry) {
            return false;
        }
        if !self.city_pieces.can_add_building(building) {
            return false;
        }
        if self.city_pieces.amount() >= player.cities.len() {
            return false;
        }
        if !player.has_advance(&building.required_advance()) {
            return false;
        }
        let cost = player.construct_cost(building, self);
        player.resources().can_afford(&cost)
    }

    pub fn can_build_wonder(&self, wonder: &Wonder, player: &Player) -> bool {
        if self.player_index != player.index {
            return false;
        }
        if self.city_pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if self.city_pieces.amount() >= player.cities.len() {
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

    pub fn conquer(mut self, game: &mut Game, new_player_index: usize, old_player_index: usize) {
        for wonder in self.city_pieces.wonders.iter() {
            (wonder.player_deinitializer)(game, old_player_index);
            (wonder.player_initializer)(game, new_player_index);
        }
        self.player_index = new_player_index;
        self.mood_state = Angry;
        for wonder in self.city_pieces.wonders.iter() {
            game.players[old_player_index].remove_wonder(wonder);
            game.players[new_player_index]
                .wonders
                .push(wonder.name.clone());
        }
        if let Some(player) = &self.city_pieces.obelisk {
            if player == &old_player_index {
                game.players[old_player_index].influenced_buildings += 1;
            }
        }
        let previously_influenced_building =
            self.city_pieces.buildings(Some(new_player_index)).len() as u32;
        self.city_pieces.change_player(new_player_index);
        let new_player = &mut game.players[new_player_index];
        new_player.influenced_buildings -= previously_influenced_building;
        new_player.cities.push(self)
    }

    pub fn raze(self, game: &mut Game, player_index: usize) {
        for wonder in self.city_pieces.wonders.into_iter() {
            (wonder.player_deinitializer)(game, player_index);
            game.players[player_index].remove_wonder(&wonder);
            let builder = &mut game.players[wonder.builder.expect("Wonder should have a builder")];
            builder.wonders_build -= 1;
        }
    }

    pub fn size(&self) -> usize {
        self.city_pieces.amount() + 1
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

    pub fn uninfluenced_buildings(&self) -> u32 {
        self.city_pieces.buildings(Some(self.player_index)).len() as u32
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CityData {
    city_pieces: CityPiecesData,
    mood_state: MoodState,
    is_activated: bool,
    player_index: usize,
    position: Position,
}

impl CityData {
    pub fn new(
        city_pieces: CityPiecesData,
        mood_state: MoodState,
        is_activated: bool,
        player_index: usize,
        position: Position,
    ) -> Self {
        Self {
            city_pieces,
            mood_state,
            is_activated,
            player_index,
            position,
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
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
        serde_json::to_string(&self).expect("city piece data should be valid json")
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).expect("API call should receive valid city piece data json")
    }

    fn required_advance(&self) -> String {
        String::from(match self {
            Self::Academy => "Writing",
            Self::Market => "Bartering",
            Self::Obelisk => "Arts",
            Self::Observatory => "Math",
            Self::Fortress => "Tactics",
            Self::Port => "Fishing",
            Self::Temple => "Myths",
        })
    }
}

#[derive(Default)]
pub struct CityPieces {
    pub academy: Option<usize>,
    pub market: Option<usize>,
    pub obelisk: Option<usize>,
    pub observatory: Option<usize>,
    pub fortress: Option<usize>,
    pub port: Option<usize>,
    pub temple: Option<usize>,
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
            Academy => self.academy.is_none(),
            Market => self.market.is_none(),
            Obelisk => self.obelisk.is_none(),
            Observatory => self.observatory.is_none(),
            Fortress => self.fortress.is_none(),
            Port => self.port.is_none(),
            Temple => self.temple.is_none(),
        }
    }

    pub fn set_building(&mut self, building: &Building, player_index: usize) {
        match building {
            Academy => self.academy = Some(player_index),
            Market => self.market = Some(player_index),
            Obelisk => self.obelisk = Some(player_index),
            Observatory => self.observatory = Some(player_index),
            Fortress => self.fortress = Some(player_index),
            Port => self.port = Some(player_index),
            Temple => self.temple = Some(player_index),
        }
    }

    fn amount(&self) -> usize {
        self.buildings(None).len() + self.wonders.len()
    }

    fn change_player(&mut self, new_player_index: usize) {
        for b in self.buildings(None) {
            if !matches!(b, Obelisk) {
                self.set_building(&b, new_player_index);
            }
        }
    }

    pub fn buildings(&self, owned_by: Option<usize>) -> Vec<Building> {
        vec![
            (Academy, self.academy),
            (Market, self.market),
            (Obelisk, self.obelisk),
            (Observatory, self.observatory),
            (Fortress, self.fortress),
            (Port, self.port),
            (Temple, self.temple),
        ]
        .into_iter()
        .filter_map(|(building, owner)| {
            owner
                .filter(|owner| match owned_by {
                    Some(want_owner) => owner == &want_owner,
                    None => true,
                })
                .map(|_| building)
        })
        .collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CityPiecesData {
    academy: Option<usize>,
    market: Option<usize>,
    obelisk: Option<usize>,
    observatory: Option<usize>,
    fortress: Option<usize>,
    port: Option<usize>,
    temple: Option<usize>,
    wonders: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum MoodState {
    Happy,
    Neutral,
    Angry,
}
