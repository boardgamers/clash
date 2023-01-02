use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GameData;

pub type GameJson = String;

pub struct Move;

pub struct GameOptions;

pub struct LogSliceOptions {
    player: Option<usize>,
    start: u32,
    end: Option<u32>,
}

pub struct LogData;

// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[export_name = "init"]
pub async extern "C" fn init(
    players: u32,
    expansions: Vec<String>,
    options: GameOptions,
    seed: String,
    creator: Option<usize>,
) -> GameJson {
    to_json(GameData {});
    todo!()
}

#[export_name = "move"]
pub extern "C" fn execute_move(json: GameJson, move_data: Move, player: usize) -> GameJson {
    let data = from_json(json);
    to_json(data);
    todo!()
}

#[export_name = "ended"]
pub extern "C" fn ended(json: GameJson) -> bool {
    let data = from_json(json);
    todo!()
}

#[export_name = "scores"]
pub extern "C" fn scores(json: GameJson) -> Vec<u32> {
    let data = from_json(json);
    todo!()
}

#[export_name = "dropPlayer"]
pub async extern "C" fn drop_player(json: GameJson, player: usize) -> GameJson {
    let data = from_json(json);
    to_json(data);
    todo!()
}

#[export_name = "currentPlayer"]
pub async extern "C" fn current_player(json: GameJson) -> Option<usize> {
    let data = from_json(json);
    todo!()
}

#[export_name = "logLength"]
pub async extern "C" fn log_length(json: GameJson) -> u32 {
    let data = from_json(json);
    todo!()
}

#[export_name = "logSlice"]
pub async extern "C" fn log_slice(json: GameJson, options: LogSliceOptions) -> LogData {
    let data = from_json(json);
    todo!()
}

fn from_json(data: GameJson) -> GameData {
    let game: GameData =
        serde_json::from_str(&data).expect("API call should receive valid game data json");
    game
}

fn to_json(game: GameData) -> GameJson {
    serde_json::to_string(&game).expect("game data should be valid json")
}
