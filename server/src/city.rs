use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::{Add, Sub};

use crate::content::custom_actions::CustomActionType::ForcedLabor;
use crate::content::persistent_events::PersistentEventType;
use crate::events::{EventOrigin, EventPlayer};
use crate::log::{ActionLogBalance, ActionLogEntry, add_action_log_item};
use crate::map::Terrain;
use crate::player::remove_unit;
use crate::structure::{Structure, log_gain_city, log_lose_city};
use crate::unit::{UnitType, Units};
use crate::utils;
use crate::wonder::destroy_wonder;
use crate::{
    city_pieces::{CityPieces, CityPiecesData},
    game::Game,
    player::Player,
    position::Position,
};
use MoodState::*;
use itertools::Itertools;
use num::Zero;
use crate::city_pieces::lose_building;

pub struct City {
    pub pieces: CityPieces,
    pub mood_state: MoodState,
    pub activations: u32,
    pub activation_mood_decreased: bool, // transient
    pub angry_activation: bool,
    pub player_index: usize,
    pub position: Position,
    pub port_position: Option<Position>,
}

impl City {
    #[must_use]
    pub fn from_data(data: CityData, player_index: usize) -> Self {
        Self {
            pieces: CityPieces::from_data(data.city_pieces),
            mood_state: data.mood_state,
            activations: data.activations,
            angry_activation: data.angry_activation,
            activation_mood_decreased: false, // transient, not in data
            player_index,
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
            activation_mood_decreased: false, // transient, not in data
            player_index,
            position,
            port_position: None,
        }
    }

    #[must_use]
    pub fn can_activate(&self) -> bool {
        !self.angry_activation
    }

    pub fn deactivate(&mut self) {
        self.activations = 0;
        self.angry_activation = false;
    }

    #[must_use]
    pub fn is_activated(&self) -> bool {
        self.activations > 0
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.pieces.amount() + 1
    }

    #[must_use]
    pub fn mood_modified_size(&self, player: &Player) -> usize {
        match self.mood_state {
            Happy => self.size() + 1,
            Neutral => self.size(),
            Angry => {
                if player.played_once_per_turn_actions.contains(&ForcedLabor) {
                    self.size()
                } else {
                    1
                }
            }
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
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CityData {
    #[serde(default)]
    #[serde(skip_serializing_if = "CityPiecesData::is_empty")]
    city_pieces: CityPiecesData,
    mood_state: MoodState,
    #[serde(default)]
    #[serde(skip_serializing_if = "u32::is_zero")]
    activations: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    angry_activation: bool,
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
        position: Position,
        port_position: Option<Position>,
    ) -> Self {
        Self {
            city_pieces,
            mood_state,
            activations,
            angry_activation,
            position,
            port_position,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Ord, PartialOrd, Eq)]
pub enum MoodState {
    Happy = 2,
    Neutral = 1,
    Angry = 0,
}

impl Display for MoodState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Happy => write!(f, "Happy"),
            Neutral => write!(f, "Neutral"),
            Angry => write!(f, "Angry"),
        }
    }
}

impl Add<u8> for MoodState {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
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

impl Sub<u8> for MoodState {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
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

pub(crate) fn is_valid_city_terrain(t: &Terrain) -> bool {
    t.is_land() && !matches!(t, Terrain::Exhausted(_) | Terrain::Barren)
}

pub(crate) fn execute_found_city_action(
    game: &mut Game,
    player_index: usize,
    settler: u32,
) -> Result<(), String> {
    let settler = remove_unit(player_index, settler, game);
    let mut u = Units::empty();
    u += &UnitType::Settler;
    let origin = EventOrigin::Ability("Found city".to_string());
    add_action_log_item(
        game,
        player_index,
        ActionLogEntry::units(u, ActionLogBalance::Loss),
        origin.clone(),
        vec![],
    );
    if !settler.can_found_city(game) {
        return Err("Cannot found city".to_string());
    }
    found_city(
        game,
        &EventPlayer::from_player(player_index, game, origin),
        settler.position,
    );
    Ok(())
}

pub(crate) fn found_city(game: &mut Game, player: &EventPlayer, position: Position) {
    player
        .get_mut(game)
        .cities
        .push(City::new(player.index, position));
    log_gain_city(game, player, Structure::CityCenter, position);
    on_found_city(game, player.index, position);
}

pub(crate) fn on_found_city(game: &mut Game, player_index: usize, position: Position) {
    let _ = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.found_city,
        position,
        PersistentEventType::FoundCity,
    );
}

#[must_use]
pub(crate) fn non_angry_cites(p: &Player) -> Vec<Position> {
    p.cities
        .iter()
        .filter(|c| !matches!(c.mood_state, MoodState::Angry))
        .map(|c| c.position)
        .collect_vec()
}

pub(crate) fn activate_city(position: Position, game: &mut Game, origin: &EventOrigin) {
    let city = game.get_any_city_mut(position);
    assert!(city.can_activate());

    if city.mood_state == Angry {
        city.angry_activation = true;
    }
    let was_activated = city.is_activated();
    city.activations += 1;
    if was_activated {
        city.activation_mood_decreased = true;
        decrease_city_mood(game, position, origin);
    }
}

pub(crate) fn decrease_city_mood(game: &mut Game, position: Position, origin: &EventOrigin) {
    set_city_mood(
        game,
        position,
        origin,
        match game.get_any_city(position).mood_state {
            Happy => Neutral,
            Neutral | Angry => Angry,
        },
    );
}

pub fn increase_mood_state(game: &mut Game, position: Position, steps: u8, origin: &EventOrigin) {
    let city = game.get_any_city_mut(position);
    city.angry_activation = false;

    let mut state = city.mood_state.clone();
    for _ in 0..steps {
        state = match state {
            Happy | Neutral => Happy,
            Angry => Neutral,
        }
    }

    set_city_mood(game, position, origin, state);
}

pub(crate) fn set_city_mood(
    game: &mut Game,
    position: Position,
    origin: &EventOrigin,
    new_state: MoodState,
) {
    let city = game.get_any_city_mut(position);
    let player = city.player_index;
    let old_state = city.mood_state.clone();
    if old_state == new_state {
        return;
    }
    city.mood_state = new_state.clone();

    game.log(
        player,
        origin,
        &format!("City {position} became {new_state}"),
    );
    add_action_log_item(
        game,
        player,
        ActionLogEntry::mood_change(position, new_state),
        origin.clone(),
        vec![],
    );
}

pub(crate) fn gain_city(game: &mut Game, player: &EventPlayer, mut city: City) {
    city.player_index = player.index;
    log_gain_city(game, player, Structure::CityCenter, city.position);
    player.get_mut(game).cities.push(city);
}

pub(crate) fn lose_city(game: &mut Game, player: &EventPlayer, position: Position) -> City {
    let p = player.get_mut(game);
    let city = if let Some(pos) = p.cities.iter().position(|city| city.position == position) {
        p.cities.remove(pos)
    } else {
        let any_city = game.try_get_any_city(position).map(|c| c.player_index);
        panic!(
            "{} should have this city {position} but does not - found owner: {:?}",
            player.index, any_city
        );
    };
    log_lose_city(game, player, Structure::CityCenter, city.position);
    city
}

pub(crate) fn raze_city(game: &mut Game, player: &EventPlayer, city: City) {
    for b in &city.pieces.buildings(None) {
        lose_building(game, player, *b, city.position);
    }
    for wonder in &city.pieces.wonders {
        destroy_wonder(game, player, *wonder, city.position);
    }
}
