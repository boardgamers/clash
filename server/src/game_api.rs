pub struct GameData;

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
pub async extern fn init(players: u32, expansions: Vec<String>, options: GameOptions, seed: String, creator: Option<usize>) -> GameData {
    todo!()
}

#[export_name = "move"]
pub extern fn execute_move(game: GameData, move_data: Move, player: usize) -> GameData {
    todo!()
}

#[export_name = "ended"]
pub extern fn ended(game: GameData) -> bool {
    todo!()
}

#[export_name = "scores"]
pub extern fn scores(game: GameData) -> Vec<u32> {
    todo!()
}

#[export_name = "dropPlayer"]
pub async extern fn drop_player(game: GameData, player: usize) -> GameData {
    todo!()
}

#[export_name = "currentPlayer"]
pub async extern fn current_player(game: GameData) -> Option<usize> {
    todo!()
}

#[export_name = "logLength"]
pub async extern fn log_length(game: GameData) -> u32 {
    todo!()
}

#[export_name = "logSlice"]
pub async extern fn log_slice(game: GameData, options: LogSliceOptions) -> LogData {
    todo!()
}
