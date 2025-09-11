#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![allow(clippy::similar_names)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::implicit_hasher)]
#![allow(clippy::struct_field_names)]
extern crate core;

pub mod ability_initializer;
pub mod action;
pub mod action_card;
mod action_cost;
pub mod advance;
#[cfg(not(target_arch = "wasm32"))]
pub mod ai;
#[cfg(not(target_arch = "wasm32"))]
pub mod ai_actions;
#[cfg(not(target_arch = "wasm32"))]
pub mod ai_collect;
#[cfg(not(target_arch = "wasm32"))]
pub mod ai_missions;
pub mod barbarians;
pub mod cache;
pub mod card;
pub mod city;
pub mod city_pieces;
pub mod civilization;
pub mod collect;
pub mod combat;
mod combat_listeners;
pub mod combat_roll;
pub mod combat_stats;
pub mod construct;
pub mod consts;
pub mod content;
pub mod cultural_influence;
pub mod events;
mod explore;
pub mod game;
pub mod game_api;
pub mod game_api_wrapper;
pub mod game_data;
pub mod game_setup;
pub mod happiness;
pub mod incident;
pub mod leader;
mod leader_ability;
pub mod log;
pub mod map;
mod move_routes;
pub mod movement;
pub mod objective_card;
pub mod payment;
pub mod pirates;
pub mod player;
pub mod player_events;
pub mod playing_actions;
pub mod position;
pub mod profiling;
pub mod recruit;
pub mod replay;
pub mod resource;
pub mod resource_pile;
mod special_advance;
pub mod status_phase;
pub mod structure;
pub mod tactics_card;
mod undo;
pub mod unit;
pub mod utils;
pub mod victory_points;
pub mod wonder;
