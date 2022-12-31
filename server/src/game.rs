use std::arch::asm;

struct Game;

enum MoveData {}

struct Options;

type PlayerIndex = u32;

struct LogSliceOptions {
    player: Option<PlayerIndex>,
    start: u32,
    end: Option<u32>,
}

struct LogData;

// Game API methods, see https://docs.boardgamers.space/guide/engine-api.html#required-methods

#[export_name = "init"]
pub async extern fn init(players: u32, expansions: Vec<str>, options: Options, seed: String, creator: Option<PlayerIndex>) -> Game {
    return Game{}
}

#[export_name = "move"]
pub extern fn execute_move(mut game: Game, move_data: MoveData, player: PlayerIndex) -> Game {
    return game;
}

#[export_name = "ended"]
pub extern fn ended(game: Game) -> bool {
    return false;
}

#[export_name = "scores"]
pub extern fn scores(game: Game) -> Vec<u32> {
    return Vec::new();
}

#[export_name = "dropPlayer"]
pub async extern fn dropPlayer(mut game: Game, player: PlayerIndex) -> Game {
    return game;
}

#[export_name = "currentPlayer"]
pub async extern fn currentPlayer(game: Game) -> Option<PlayerIndex> {
    return Option::None;
}

#[export_name = "logLength"]
pub async extern fn logLength(game: Game) -> u32 {
    return 0;
}

#[export_name = "logSlice"]
pub async extern fn logSlice(game: Game, options: LogSliceOptions) -> LogData {
    return LogData;
}










