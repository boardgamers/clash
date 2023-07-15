use serde::{Deserialize, Serialize};

use crate::city_pieces::Building::*;
use crate::{
    city_pieces::{Building, CityPieces, CityPiecesData},
    game::Game,
    player::Player,
    position::Position,
    wonder::Wonder,
};
use MoodState::*;

const MAX_CITY_SIZE: usize = 4;

pub struct City {
    pub city_pieces: CityPieces,
    pub mood_state: MoodState,
    pub is_activated: bool,
    activations: u32,
    pub player_index: usize,
    pub position: Position,
}

impl City {
    pub fn from_data(data: CityData) -> Self {
        Self {
            city_pieces: CityPieces::from_data(data.city_pieces),
            mood_state: data.mood_state,
            is_activated: data.is_activated,
            activations: data.activations,
            player_index: data.player_index,
            position: data.position,
        }
    }

    pub fn data(self) -> CityData {
        CityData::new(
            self.city_pieces.data(),
            self.mood_state,
            self.is_activated,
            self.activations,
            self.player_index,
            self.position,
        )
    }

    pub fn new(player_index: usize, position: Position) -> Self {
        Self {
            city_pieces: CityPieces::default(),
            mood_state: Neutral,
            is_activated: false,
            activations: 0,
            player_index,
            position,
        }
    }

    pub fn activate(&mut self) {
        if self.is_activated() {
            self.decrease_mood_state();
        }
        self.activations += 1;
    }

    pub fn deactivate(&mut self) {
        self.activations = 0;
    }

    pub fn undo_activate(&mut self) {
        self.activations -= 1;
        if self.is_activated() {
            self.increase_mood_state();
        }
    }

    pub fn is_activated(&self) -> bool {
        self.activations > 0
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
        if !player.available_buildings.can_build(building) {
            return false;
        }
        let cost = player.construct_cost(building, self);
        player.resources().can_afford(&cost)
    }

    pub fn can_build_wonder(&self, wonder: &Wonder, player: &Player, game: &Game) -> bool {
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
        if let Some(placement_requirement) = &wonder.placement_requirement {
            return placement_requirement(&self.position, game);
        }
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
        for (building, owner) in self.city_pieces.building_owners() {
            if matches!(building, Obelisk) {
                continue;
            }
            let Some(owner) = owner else {
                continue;
            };
            if owner != old_player_index {
                continue;
            }
            self.city_pieces.set_building(&building, new_player_index);
            game.players[old_player_index].available_buildings += &building;
            game.players[new_player_index].available_buildings -= &building;
        }
        let new_player = &mut game.players[new_player_index];
        new_player.influenced_buildings -= previously_influenced_building;
        new_player.cities.push(self)
    }

    pub fn raze(self, game: &mut Game, player_index: usize) {
        for (building, owner) in self.city_pieces.building_owners().iter() {
            if let Some(owner) = owner {
                game.players[*owner].available_buildings += building;
            }
        }
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

    pub fn influenced(&self) -> bool {
        self.uninfluenced_buildings() as usize != self.city_pieces.amount()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CityData {
    city_pieces: CityPiecesData,
    mood_state: MoodState,
    is_activated: bool,
    activations: u32,
    player_index: usize,
    position: Position,
}

impl CityData {
    pub fn new(
        city_pieces: CityPiecesData,
        mood_state: MoodState,
        is_activated: bool,
        activations: u32,
        player_index: usize,
        position: Position,
    ) -> Self {
        Self {
            city_pieces,
            mood_state,
            is_activated,
            activations,
            player_index,
            position,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum MoodState {
    Happy = 2,
    Neutral = 1,
    Angry = 0,
}
