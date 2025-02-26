#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::similar_names)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::implicit_hasher)]

pub mod ability_initializer;
pub mod action;
pub mod advance;
mod barbarians;
pub mod city;
pub mod city_pieces;
mod civilization;
pub mod collect;
pub mod combat;
mod combat_listeners;
pub mod consts;
pub mod content;
pub mod cultural_influence;
pub mod events;
mod explore;
pub mod game;
pub mod game_api;
pub mod game_api_wrapper;
pub mod incident;
pub mod leader;
pub mod log;
pub mod map;
pub mod move_units;
mod movement;
pub mod payment;
mod pirates;
pub mod player;
pub mod player_events;
pub mod playing_actions;
pub mod position;
pub mod recruit;
pub mod resource;
pub mod resource_pile;
mod special_advance;
pub mod status_phase;
mod undo;
pub mod unit;
mod utils;
mod wonder;
