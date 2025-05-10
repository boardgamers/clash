#![allow(clippy::pedantic)]

extern crate console_error_panic_hook;
use crate::cache::Cache;
use crate::game::{GameContext, GameOptions};
use crate::{game::Game, game_api};
use serde::{Deserialize, Serialize};
use std::mem;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct PlayerMetaData {
    name: String,
}

fn get_game(data: String) -> Game {
    console_error_panic_hook::set_once();
    Game::from_data(
        serde_json::from_str(&data).expect("Could not deserialize game data"),
        Cache::new(),
        GameContext::Server,
    )
}

fn from_game(game: Game) -> String {
    serde_json::to_string(&game.data()).expect("game should be serializable")
}

#[wasm_bindgen]
pub async fn init(
    player_amount: usize,
    _expansions: JsValue,
    options: JsValue,
    seed: String,
    _creator: JsValue,
) -> String {
    let options = serde_wasm_bindgen::from_value::<GameOptions>(options)
        .expect("options should be serializable");
    let game = game_api::init(player_amount, seed, options);
    from_game(game)
}

#[wasm_bindgen(js_name = move)]
pub fn execute_move(game: String, move_data: String, player_index: usize) -> String {
    let game = get_game(game);
    let action = serde_json::from_str(&move_data).expect("move should be of type action");
    let game = game_api::execute(game, action, player_index);
    from_game(game)
}

#[wasm_bindgen]
pub fn ended(game: String) -> JsValue {
    let game = get_game(game);
    JsValue::from_bool(game_api::ended(&game))
}

#[wasm_bindgen]
pub fn scores(game: String) -> JsValue {
    let game = get_game(game);
    let scores = game_api::scores(&game);
    serde_wasm_bindgen::to_value(&scores).expect("scores should be serializable")
}

#[wasm_bindgen(js_name = "dropPlayer")]
pub async fn drop_player(game: String, player_index: usize) -> String {
    let game = get_game(game);
    let game = game_api::drop_player(game, player_index);
    from_game(game)
}

#[wasm_bindgen(js_name = "currentPlayer")]
pub fn current_player(game: String) -> JsValue {
    let game = get_game(game);
    JsValue::from_f64(game.active_player() as f64)
}

#[wasm_bindgen(js_name = "logLength")]
pub fn log_length(game: String) -> JsValue {
    let game = get_game(game);
    let log_length = game_api::log_length(&game);
    JsValue::from_f64(log_length as f64)
}

#[wasm_bindgen(js_name = "logSlice")]
pub fn log_slice(game: String, options: JsValue) -> JsValue {
    let game = get_game(game);
    let options = serde_wasm_bindgen::from_value(options).expect("options should be serializable");
    let log = game_api::log_slice(&game, &options);
    serde_wasm_bindgen::to_value(&log).expect("log should be serializable")
}

#[wasm_bindgen(js_name = "setPlayerMetaData")]
pub fn set_player_meta_data(game: String, player_index: usize, meta_data: JsValue) -> String {
    let game = get_game(game);
    let name = serde_wasm_bindgen::from_value::<PlayerMetaData>(meta_data)
        .expect("meta data should be of type player meta data")
        .name;
    let game = game_api::set_player_name(game, player_index, name);
    from_game(game)
}

#[wasm_bindgen]
pub fn rankings(game: String) -> JsValue {
    let game = get_game(game);
    let rankings = game_api::rankings(&game);
    serde_wasm_bindgen::to_value(&rankings).expect("rankings should be serializable")
}

#[wasm_bindgen(js_name = "round")]
pub fn round_number(game: String) -> JsValue {
    let game = get_game(game);
    JsValue::from_f64(game_api::round(&game) as f64)
}

#[wasm_bindgen]
pub fn factions(game: String) -> JsValue {
    let game = get_game(game);
    let factions = game_api::civilizations(game);
    serde_wasm_bindgen::to_value(&factions).expect("faction list should be serializable")
}

#[wasm_bindgen(js_name = "stripSecret")]
pub fn strip_secret(game: String, player_index: Option<usize>) -> String {
    let game = get_game(game);
    let game = game_api::strip_secret(game, player_index);
    from_game(game)
}

#[wasm_bindgen]
pub fn messages(game: String) -> JsValue {
    let mut game = get_game(game);
    let messages = Messages::new(mem::take(&mut game.messages), from_game(game));
    serde_wasm_bindgen::to_value(&messages).expect("messages should be serializable")
}

#[derive(Serialize, Deserialize)]
pub struct Messages {
    messages: Vec<String>,
    data: String,
}

impl Messages {
    #[must_use]
    pub fn new(messages: Vec<String>, data: String) -> Self {
        Self { messages, data }
    }
}
