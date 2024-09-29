#![allow(clippy::pedantic)]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{game::Game, game_api};
extern crate console_error_panic_hook;
use std::panic;

#[derive(Serialize, Deserialize)]
pub struct PlayerMetaData {
    name: String,
}

fn get_game(data: JsValue) -> Game {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    Game::from_data(serde_wasm_bindgen::from_value(data).expect("game should be of type game data"))
}

fn from_game(game: Game) -> JsValue {
    serde_wasm_bindgen::to_value(&game.data()).expect("game should be serializable")
}

#[wasm_bindgen]
pub async fn init(
    players: JsValue,
    _expansions: JsValue,
    _options: JsValue,
    seed: JsValue,
    _creator: JsValue,
) -> JsValue {
    let player_amount = players
        .as_f64()
        .expect("number of players should be a number") as usize;
    let seed = seed.as_string().expect("seed should be a string");
    let game = game_api::init(player_amount, seed);
    from_game(game)
}

#[wasm_bindgen(js_name = move)]
pub fn execute_move(game: JsValue, move_data: JsValue, player: JsValue) -> JsValue {
    let game = get_game(game);
    let action = serde_wasm_bindgen::from_value(move_data).expect("move should be of type action");
    let player_index = player.as_f64().expect("player index should be a number") as usize;
    let game = game_api::execute_action(game, action, player_index);
    from_game(game)
}

#[wasm_bindgen]
pub fn ended(game: JsValue) -> JsValue {
    let game = get_game(game);
    JsValue::from_bool(game_api::ended(&game))
}

#[wasm_bindgen]
pub fn scores(game: JsValue) -> JsValue {
    let game = get_game(game);
    let scores = game_api::scores(&game);
    serde_wasm_bindgen::to_value(&scores).expect("scores should be serializable")
}

#[wasm_bindgen(js_name = "dropPlayer")]
pub async fn drop_player(game: JsValue, player: JsValue) -> JsValue {
    let game = get_game(game);
    let player_index = player.as_f64().expect("player index should be a number") as usize;
    let game = game_api::drop_player(game, player_index);
    from_game(game)
}

#[wasm_bindgen(js_name = "currentPlayer")]
pub fn current_player(game: JsValue) -> JsValue {
    let game = get_game(game);
    let player_index = game_api::current_player(&game);
    match player_index {
        Some(index) => JsValue::from_f64(index as f64),
        None => JsValue::undefined(),
    }
}

#[wasm_bindgen(js_name = "logLength")]
pub fn log_length(game: JsValue) -> JsValue {
    let game = get_game(game);
    let log_length = game_api::log_length(&game);
    JsValue::from_f64(log_length as f64)
}

#[wasm_bindgen(js_name = "logSlice")]
pub fn log_slice(game: JsValue, options: JsValue) -> JsValue {
    let game = get_game(game);
    let options = serde_wasm_bindgen::from_value(options).expect("options should be serializable");
    let log = game_api::log_slice(&game, &options);
    serde_wasm_bindgen::to_value(&log).expect("log should be serializable")
}

#[wasm_bindgen(js_name = "setPlayerMetaData")]
pub fn set_player_meta_data(game: JsValue, player_index: JsValue, meta_data: JsValue) -> JsValue {
    let game = get_game(game);
    let player_index = player_index
        .as_f64()
        .expect("player index should be a number") as usize;
    let name = serde_wasm_bindgen::from_value::<PlayerMetaData>(meta_data)
        .expect("meta data should be of type player meta data")
        .name;
    let game = game_api::set_player_name(game, player_index, name);
    from_game(game)
}

#[wasm_bindgen]
pub fn rankings(game: JsValue) -> JsValue {
    let game = get_game(game);
    let rankings = game_api::rankings(&game);
    serde_wasm_bindgen::to_value(&rankings).expect("rankings should be serializable")
}

#[wasm_bindgen(js_name = "roundNumber")]
pub fn round_number(game: JsValue) -> JsValue {
    let game = get_game(game);
    let round = game_api::round(&game);
    match round {
        Some(round) => JsValue::from_f64(round as f64),
        None => JsValue::undefined(),
    }
}

#[wasm_bindgen]
pub fn factions(game: JsValue) -> JsValue {
    let game = get_game(game);
    let factions = game_api::civilizations(game);
    serde_wasm_bindgen::to_value(&factions).expect("faction list should be serializable")
}

#[wasm_bindgen(js_name = "stripSecret")]
pub fn strip_secret(game: JsValue, player: JsValue) -> JsValue {
    let game = get_game(game);
    let player_index = player.as_f64().map(|player_index| player_index as usize);
    let game = game_api::strip_secret(game, player_index);
    from_game(game)
}

#[wasm_bindgen]
pub fn messages(game: JsValue) -> JsValue {
    let game = get_game(game);
    let messages = game_api::messages(game);
    serde_wasm_bindgen::to_value(&messages).expect("messages should be serializable")
}
