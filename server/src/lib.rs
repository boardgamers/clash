#![warn(clippy::pedantic)]
#![allow(clippy::similar_names)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]

mod ability_initializer;
pub mod action;
mod advance;
pub mod city;
pub mod city_pieces;
mod civilization;
pub mod consts;
pub mod content;
mod events;
pub mod game;
pub mod game_api;
pub mod game_api_wrapper;
pub mod leader;
pub mod log;
pub mod map;
pub mod player;
mod player_events;
pub mod playing_actions;
pub mod position;
pub mod resource_pile;
mod special_advance;
mod status_phase;
pub mod unit;
mod utils;
mod wonder;
