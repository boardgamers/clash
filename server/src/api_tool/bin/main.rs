#![allow(clippy::missing_panics_doc)]

use serde::Serialize;
use server::game::Game;
use server::replay;
use server::replay::ReplayGameData;
use std::fs::File;
use std::time::SystemTime;
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} command", args[0]);
        return;
    }
    let command = &args[1];
    match command.as_str() {
        "replay" => {
            replay(args.get(2));
        }
        _ => {
            println!("Unknown command: {command}");
        }
    }
}

fn replay(to: Option<&String>) {
    let data: ReplayGameData =
        serde_json::from_str(&read_game_str()).expect("Failed to read export file");
    let game = replay::replay(
        data,
        to.map(|s| s.parse::<usize>().expect("Failed to parse replay index")),
    );
    export(game)
}

fn read_game_str() -> String {
    // read from game.json instead of escaped-game.json if the modification date is newer
    let g = "game.json";
    let e = "escaped-game.json";
    if modified(g) > modified(e) {
        return fs::read_to_string(g).expect("Failed to read export file");
    }
    let val: String =
        serde_json::from_str(&fs::read_to_string(e).expect("Failed to read export file"))
            .expect("Failed to parse export file");
    fs::write(g, &val).expect("Failed to write export file");
    val
}

fn modified(g: &str) -> SystemTime {
    fs::metadata(g).unwrap().modified().unwrap()
}

fn export(game: Game) {
    let data = game.data();

    write(
        &serde_json::to_string(&data).expect("Failed to serialize game"),
        "escaped-game-out.json",
    );
    write(&data, "game-out.json");
}

fn write<T: Serialize>(data: &T, path: &str) {
    serde_json::to_writer_pretty(
        File::create(path).expect("Failed to create export file"),
        &data,
    )
    .expect("Failed to write export file");
}
