use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use crate::consts::MAX_CITY_SIZE;
use crate::resource_pile::ResourcePile;
use crate::{
    city_pieces::{Building, CityPieces, CityPiecesData},
    game::Game,
    player::Player,
    position::Position,
    wonder::Wonder,
};
use MoodState::*;

pub struct City {
    pub pieces: CityPieces,
    pub mood_state: MoodState,
    pub activations: u32,
    pub angry_activation: bool,
    pub player_index: usize,
    pub position: Position,
    pub port_position: Option<Position>,
}

impl City {
    #[must_use]
    pub fn from_data(data: CityData) -> Self {
        Self {
            pieces: CityPieces::from_data(&data.city_pieces),
            mood_state: data.mood_state,
            activations: data.activations,
            angry_activation: data.angry_activation,
            player_index: data.player_index,
            position: data.position,
            port_position: data.port_position,
        }
    }

    #[must_use]
    pub fn data(self) -> CityData {
        CityData::new(
            self.pieces.data(),
            self.mood_state,
            self.activations,
            self.angry_activation,
            self.player_index,
            self.position,
            self.port_position,
        )
    }

    #[must_use]
    pub fn cloned_data(&self) -> CityData {
        CityData::new(
            self.pieces.cloned_data(),
            self.mood_state.clone(),
            self.activations,
            self.angry_activation,
            self.player_index,
            self.position,
            self.port_position,
        )
    }

    #[must_use]
    pub fn new(player_index: usize, position: Position) -> Self {
        Self {
            pieces: CityPieces::default(),
            mood_state: Neutral,
            activations: 0,
            angry_activation: false,
            player_index,
            position,
            port_position: None,
        }
    }

    #[must_use]
    pub fn can_activate(&self) -> bool {
        !self.angry_activation
    }

    pub fn activate(&mut self) {
        if self.mood_state == Angry {
            self.angry_activation = true;
        }
        if self.is_activated() {
            self.decrease_mood_state();
        }
        self.activations += 1;
    }

    pub fn deactivate(&mut self) {
        self.activations = 0;
        self.angry_activation = false;
    }

    pub fn undo_activate(&mut self) {
        self.activations -= 1;
        self.angry_activation = false;
        if self.is_activated() {
            self.increase_mood_state();
        }
    }

    #[must_use]
    pub fn is_activated(&self) -> bool {
        self.activations > 0
    }

    #[must_use]
    pub fn can_construct(&self, building: Building, player: &Player, game: &Game) -> bool {
        if self.player_index != player.index {
            return false;
        }
        if self.pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if matches!(self.mood_state, Angry) {
            return false;
        }
        if !self.pieces.can_add_building(building) {
            return false;
        }
        let size = self.pieces.amount() + 1;
        if size >= player.cities.len() {
            return false;
        }
        if !player.has_advance(&building.required_advance()) {
            return false;
        }
        if !player.is_building_available(building, game) {
            return false;
        }
        let cost = player.construct_cost(building, self);
        player.resources.can_afford(&cost)
    }

    #[must_use]
    pub fn can_build_wonder(&self, wonder: &Wonder, player: &Player, game: &Game) -> bool {
        if self.player_index != player.index {
            return false;
        }
        if self.pieces.amount() == MAX_CITY_SIZE {
            return false;
        }
        if self.pieces.amount() >= player.cities.len() {
            return false;
        }
        if !matches!(self.mood_state, Happy) {
            return false;
        }
        let cost = player.wonder_cost(wonder, self);
        if !player.resources.can_afford(&cost) {
            return false;
        }
        for advance in &wonder.required_advances {
            if !player.has_advance(advance) {
                return false;
            }
        }
        if let Some(placement_requirement) = &wonder.placement_requirement {
            return placement_requirement(self.position, game);
        }
        true
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if the city does not have a builder
    pub fn raze(self, game: &mut Game, player_index: usize) {
        for wonder in &self.pieces.wonders {
            (wonder.player_deinitializer)(game, player_index);
        }
        for wonder in &self.pieces.wonders {
            for p in &mut game.players {
                p.remove_wonder(wonder);
            }
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.pieces.amount() + 1
    }

    #[must_use]
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
        };
        self.angry_activation = false;
    }

    pub fn decrease_mood_state(&mut self) {
        self.mood_state = match self.mood_state {
            Happy => Neutral,
            Neutral | Angry => Angry,
        }
    }

    #[must_use]
    fn uninfluenced_buildings(&self) -> u32 {
        self.pieces.buildings(Some(self.player_index)).len() as u32
    }

    #[must_use]
    pub fn influenced(&self) -> bool {
        self.uninfluenced_buildings() as usize != self.pieces.amount()
    }

    #[must_use]
    pub fn increase_happiness_cost(&self, steps: u32) -> Option<ResourcePile> {
        let max_steps = 2 - self.mood_state.clone() as u32;
        if steps > max_steps {
            None
        } else {
            Some(ResourcePile::mood_tokens(self.size() as u32) * steps)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CityData {
    city_pieces: CityPiecesData,
    mood_state: MoodState,
    activations: u32,
    angry_activation: bool,
    player_index: usize,
    position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    port_position: Option<Position>,
}

impl CityData {
    #[must_use]
    pub fn new(
        city_pieces: CityPiecesData,
        mood_state: MoodState,
        activations: u32,
        angry_activation: bool,
        player_index: usize,
        position: Position,
        port_position: Option<Position>,
    ) -> Self {
        Self {
            city_pieces,
            mood_state,
            activations,
            angry_activation,
            player_index,
            position,
            port_position,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum MoodState {
    Happy = 2,
    Neutral = 1,
    Angry = 0,
}

impl Add<u32> for MoodState {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        match rhs {
            0 => self,
            1 => match self {
                Happy | Neutral => Happy,
                Angry => Neutral,
            },
            2.. => Happy,
        }
    }
}

impl Sub<u32> for MoodState {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        match rhs {
            0 => self,
            1 => match self {
                Angry | Neutral => Angry,
                Happy => Neutral,
            },
            2.. => Angry,
        }
    }
}
