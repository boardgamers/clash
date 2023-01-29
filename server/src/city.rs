use std::fmt::Display;

use crate::{content::wonders, game::Game, hexagon::Position, player::Player, wonder::Wonder};
use strum_macros::{Display, EnumIter, ToString};

use serde::{Deserialize, Serialize};
use MoodState::*;

const MAX_CITY_SIZE: u32 = 4;

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

    pub fn can_increase_size(&self, building: &Building, player: &Player) -> bool {
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
        let cost = player.building_cost(building, self);
        player.resources().can_afford(&cost)
    }

    pub fn can_build_wonder(&self, wonder: &Wonder, player: &Player) -> bool {
        if self.player_index != player.index {
            return false;
        }
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

#[derive(Debug, EnumIter, Display)]
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
            Building::Academy => self.academy.is_none(),
            Building::Market => self.market.is_none(),
            Building::Obelisk => self.obelisk.is_none(),
            Building::Observatory => self.observatory.is_none(),
            Building::Fortress => self.fortress.is_none(),
            Building::Port => self.port.is_none(),
            Building::Temple => self.temple.is_none(),
        }
    }

    pub fn set_building(&mut self, building: &Building, player_index: usize) {
        match building {
            Building::Academy => self.academy = Some(player_index),
            Building::Market => self.market = Some(player_index),
            Building::Obelisk => self.obelisk = Some(player_index),
            Building::Observatory => self.observatory = Some(player_index),
            Building::Fortress => self.fortress = Some(player_index),
            Building::Port => self.port = Some(player_index),
            Building::Temple => self.temple = Some(player_index),
        }
    }

    fn amount(&self) -> u32 {
        (self.buildings(None).len() + self.wonders.len()) as u32
    }

    fn change_player(&mut self, new_player_index: usize) {
        for b in self.buildings(None) {
            if !matches!(b, Building::Obelisk) {
                self.set_building(&b, new_player_index.clone());
            }
        }
    }

    pub fn buildings(&self, owned_by: Option<usize>) -> Vec<Building> {
        vec![
            (Building::Academy, self.academy),
            (Building::Market, self.market),
            (Building::Obelisk, self.obelisk),
            (Building::Observatory, self.observatory),
            (Building::Fortress, self.fortress),
            (Building::Port, self.port),
            (Building::Temple, self.temple),
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

#[cfg(test)]
mod tests {
    use super::{Building, City, MoodState};
    use crate::content::civilizations;
    use crate::game;
    use crate::hexagon::Position;
    use crate::player::Player;
    use crate::resource_pile::ResourcePile;
    use crate::wonder::Wonder;

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", ResourcePile::empty(), vec![]).build();
        let mut game = game::tests::test_game();
        game.players.push(old);
        game.players.push(new);
        let old = 0;
        let new = 1;

        let position = Position::new(0, 0);
        game.players[old]
            .cities
            .push(City::new(old, position));
        game.build_wonder(wonder, &position, old);
        game.players[old].increase_size(&Building::Academy, &position);
        game.players[old].increase_size(&Building::Obelisk, &position);

        assert_eq!(6.0, game.players[old].victory_points());

        game.conquer_city(&position, new, old);

        let c = game.players[new].get_city_mut(&position).unwrap();
        assert_eq!(1, c.player_index);
        assert_eq!(MoodState::Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        assert_eq!(3.0, old.victory_points());
        assert_eq!(3.0, new.victory_points());
        assert_eq!(0, old.wonders.len());
        assert_eq!(1, new.wonders.len());
        assert_eq!(1, old.influenced_buildings);
        assert_eq!(0, new.influenced_buildings);
    }
}
