#![allow(clippy::pedantic)]

extern crate console_error_panic_hook;
use crate::cache::Cache;
use crate::game::{GameContext, GameOptions};
use crate::game_data::GameData;
use crate::replay::ReplayGameData;
use crate::{game::Game, game_api, replay};
use serde::{Deserialize, Serialize};
use std::mem;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct PlayerMetaData {
    name: String,
}

fn get_game(data: String) -> Game {
    console_error_panic_hook::set_once();
    let game_data: GameData = serde_json::from_str(&data).expect("Could not deserialize game data");
    let cache = Cache::new(&game_data.options);
    Game::from_data(game_data, cache, GameContext::Play)
}

fn from_game(game: Game) -> String {
    serde_json::to_string(&game.data()).expect("game should be serializable")
}

// BGS saves these separately from moves. Do not advance the game here: its
// settings endpoint does not update the active player, timers or move log.
#[wasm_bindgen(js_name = "setPlayerSettings")]
pub fn set_player_settings(
    data: String,
    player: usize,
    settings: JsValue,
) -> Result<String, JsValue> {
    let settings: crate::player::PlayerSettings =
        serde_wasm_bindgen::from_value(settings).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut game = get_game(data);
    let p = game
        .players
        .get_mut(player)
        .filter(|p| p.is_human())
        .ok_or_else(|| JsValue::from_str("Invalid player"))?;
    if settings.skip_raze_city.is_some() {
        p.settings.skip_raze_city = settings.skip_raze_city;
    }
    Ok(from_game(game))
}

#[wasm_bindgen(js_name = "playerSettings")]
pub fn player_settings(data: String, player: usize) -> Result<JsValue, JsValue> {
    let game = get_game(data);
    let p = game
        .players
        .get(player)
        .filter(|p| p.is_human())
        .ok_or_else(|| JsValue::from_str("Invalid player"))?;
    serde_wasm_bindgen::to_value(&p.settings).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Deserialize)]
pub struct AnalysisOptions {
    player: Option<usize>,
    seed: String,
}

#[wasm_bindgen(js_name = "canLaunchAnalysisMode")]
pub fn can_launch_analysis_mode(data: String) -> bool {
    let game = get_game(data);
    game_api::ended(&game) || crate::analysis::can_create(&game)
}

#[wasm_bindgen(js_name = "createAnalysisScenario")]
pub fn create_analysis_scenario(data: String, options: JsValue) -> Result<String, JsValue> {
    let options: AnalysisOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let game = crate::analysis::create(get_game(data), options.player, &options.seed)
        .map_err(|e| JsValue::from_str(&e))?;
    // Canonical object ordering also covers custom_data maps.
    let value = serde_json::to_value(game.data()).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&value).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Deserialize)]
pub struct CreateAnalysisOptions {
    to: usize,
}

#[wasm_bindgen(js_name = "createAnalysis")]
pub fn create_analysis(data: String, options: JsValue) -> Result<String, JsValue> {
    let options: CreateAnalysisOptions =
        serde_wasm_bindgen::from_value(options).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut game = get_game(data);
    if options.to != game_api::log_length(&game) {
        return Err(JsValue::from_str(
            "Clash analysis starts from the current position; historical board snapshots cannot be used to simulate hidden cards.",
        ));
    }
    game.board_history = Default::default();
    for player in &mut game.players {
        player.settings = Default::default();
    }
    Ok(from_game(game))
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
    let game = crate::board_history::execute(game, action, player_index)
        .expect("could not execute action");
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
    if game.civilization_draft.is_some() {
        serde_wasm_bindgen::to_value(&game.active_players()).expect("players should serialize")
    } else {
        JsValue::from_f64(game.active_player() as f64)
    }
}

#[wasm_bindgen(js_name = "canMoveOutOfTurn")]
pub fn can_move_out_of_turn(data: String, move_data: JsValue, player: usize) -> bool {
    let game = get_game(data);
    let Some(draft) = &game.civilization_draft else {
        return false;
    };
    if !draft.ready.get(player).copied().unwrap_or(false) || game.dropped_players.contains(&player)
    {
        return false;
    }
    let Some(raw) = move_data.as_string() else {
        return false;
    };
    match serde_json::from_str::<crate::action::Action>(&raw) {
        Ok(crate::action::Action::ChooseCivilization(name)) => draft.offers[player].contains(&name),
        _ => false,
    }
}

#[wasm_bindgen(js_name = "isLiveUpdate")]
pub fn is_live_update(data: String) -> bool {
    get_game(data)
        .civilization_draft
        .is_some_and(|draft| draft.live_update)
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

#[derive(Serialize, Deserialize, PartialEq)]
struct ReplayOptions {
    to: Option<usize>,
}

#[wasm_bindgen(js_name = "replay")]
pub fn replay(game: String, options: JsValue) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let r: ReplayGameData = serde_json::from_str(&game).map_err(|e| {
        JsValue::from_str(&format!(
            "Cannot replay this save: supported action_log history is required ({e})"
        ))
    })?;
    let to = serde_wasm_bindgen::from_value::<ReplayOptions>(options)
        .map_err(|e| JsValue::from_str(&format!("Invalid replay options: {e}")))?
        .to;
    let game = replay::replay(r, to).map_err(|e| JsValue::from_str(&e))?;
    Ok(from_game(game))
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

#[wasm_bindgen(js_name = "webView")]
pub fn web_view(data: String, player: Option<usize>) -> Result<String, JsValue> {
    serde_json::to_string(&crate::web_view::view(&get_game(data), player))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "webCollectPreview")]
pub fn web_collect_preview(
    data: String,
    player: usize,
    city: String,
    selections: String,
) -> Result<String, JsValue> {
    let selections =
        serde_json::from_str(&selections).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let city = serde_json::from_value(serde_json::Value::String(city))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = crate::web_view::collect_preview(&get_game(data), player, city, selections)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "tryMove")]
pub fn try_move(data: String, action: String, player: usize) -> Result<String, JsValue> {
    let action = serde_json::from_str(&action).map_err(|e| JsValue::from_str(&e.to_string()))?;
    crate::board_history::execute(get_game(data), action, player)
        .map(from_game)
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen(js_name = "webRecruitPreview")]
pub fn web_recruit_preview(
    data: String,
    player: usize,
    city: String,
    units: String,
) -> Result<String, JsValue> {
    let units = serde_json::from_str(&units).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let city = serde_json::from_value(serde_json::Value::String(city))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = crate::web_view::recruit_preview(&get_game(data), player, city, units)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "webQuery")]
pub fn web_query(data: String, player: usize, query: String) -> Result<String, JsValue> {
    let query = serde_json::from_str(&query).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result = crate::web_view::query(&get_game(data), player, query)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
