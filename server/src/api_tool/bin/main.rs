#![allow(clippy::missing_panics_doc)]


use server::game::Game;
use server::replay;
use server::replay::ReplayGameData;
use std::{env, fs};
use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} command", args[0]);
        return;
    }
    let command = &args[1];
    match command.as_str() {
        "replay" => {
            replay();
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
}

fn replay() {
    let data: ReplayGameData = serde_json::from_str(&read_game_str()).expect("Failed to read export file");
    let game = replay::replay(data, None);
    export(game)
}

const EXPORT_FILE: &str = "escaped-game.json";

fn read_game_str() -> String {
    let escaped = fs::read_to_string(EXPORT_FILE).expect("Failed to read export file");
    let val: String = serde_json::from_str(&escaped).expect("Failed to parse export file");
    val
}

fn export(game: Game) {
    let string = serde_json::to_string(&game.data()).expect("Failed to serialize game");
    serde_json::to_writer_pretty(
        File::create(EXPORT_FILE).expect("Failed to create export file"),
        &string,
    )
    .expect("Failed to write export file");
}
