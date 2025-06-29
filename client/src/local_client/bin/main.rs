#![allow(clippy::missing_panics_doc)]

use client::client::{Features, GameSyncRequest, GameSyncResult, init, render_and_update};
use client::client_state::State;
use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::{next_frame, screen_width, vec2};
use macroquad::window::screen_height;
use server::action::execute_action;
use server::game::{CivSetupOption, Game, GameContext, GameOptions, UndoOption};
use server::game_data::GameData;
use server::game_setup::{GameSetupBuilder, setup_game};
use server::profiling::start_profiling;
use std::fs::File;
use std::io::BufReader;
use std::{env, vec};

#[derive(PartialEq)]
enum Mode {
    Local,
    AI,
}

#[macroquad::main("Clash")]
async fn main() {
    start_profiling();

    set_window_size(1200, 600);

    let mut args: Vec<String> = env::args().collect();
    args.remove(0); // program name
    let players = args
        .remove(0)
        .parse()
        .expect("Please provide the number of players as the first argument");
    let modes = get_modes(&args);

    let mut features = Features {
        import_export: true,
        assets_url: "assets/".to_string(),
        ai: modes.contains(&Mode::AI),
    };

    let seed = if args.len() > 2 {
        args[2].parse().unwrap()
    } else {
        "a".repeat(32)
    };
    let game = setup_game(
        &GameSetupBuilder::new(players)
            .seed(seed)
            .options(GameOptions {
                undo: UndoOption::SamePlayer,
                civilization: CivSetupOption::Select,
            })
            .build(),
    );

    run(game, &mut features).await;
}

fn get_modes(args: &[String]) -> Vec<Mode> {
    match args.first() {
        Some(arg) => match arg.as_str() {
            "generate" => vec![Mode::Local],
            "ai" => vec![Mode::AI, Mode::Local],
            _ => {
                panic!("Unknown argument: {arg}");
            }
        },
        _ => vec![Mode::Local],
    }
}

async fn run(mut game: Game, features: &mut Features) {
    let mut state = init(features).await;

    start_ai(&mut game, features, &mut state);

    let mut sync_result = GameSyncResult::None;
    state.show_player = game.active_player();
    loop {
        state.control_player = Some(game.active_player());
        state.screen_size = vec2(screen_width(), screen_height());

        let message = render_and_update(&game, &mut state, &sync_result, features);
        sync_result = GameSyncResult::None;
        match message {
            GameSyncRequest::None => {}
            GameSyncRequest::StartAutoplay => {
                game = ai_autoplay(game, features, &mut state);
                state.show_player = game.active_player();
                sync_result = GameSyncResult::Update;
            }
            GameSyncRequest::ExecuteAction(a) => {
                let p = game.active_player();
                game = execute_action(game, a, p);
                game = ai_autoplay(game, features, &mut state);
                state.show_player = game.active_player();
                sync_result = GameSyncResult::Update;
            }
            GameSyncRequest::Import => {
                game = import(game);
                state.show_player = game.active_player();
                sync_result = GameSyncResult::Update;
            }
            GameSyncRequest::Export => {
                export(&game);
            }
        };
        next_frame().await;
    }
}

#[cfg(target_arch = "wasm32")]
fn start_ai(_: &mut Game, _: &mut Features, _: &mut State) {}

#[cfg(not(target_arch = "wasm32"))]
fn start_ai(game: &mut Game, features: &mut Features, state: &mut State) {
    use server::ai::AI;

    if features.ai {
        state.ai_players = game
            .human_players(0)
            .into_iter()
            .map(|p| AI::new(1., std::time::Duration::from_secs(5), false, game, p))
            .collect()
    }
}

#[cfg(target_arch = "wasm32")]
fn ai_autoplay(game: Game, _: &mut Features, _: &mut State) -> Game {
    game
}

#[cfg(not(target_arch = "wasm32"))]
fn ai_autoplay(mut game: Game, f: &mut Features, state: &mut State) -> Game {
    if f.ai {
        while state.ai_autoplay && game.state != server::game::GameState::Finished {
            let active_player = game.active_player();
            let ai = &mut state.ai_players[active_player];
            let action = ai.next_action(&game);
            let player_index = game.active_player();
            game = execute_action(game, action, player_index);
            export(&game)
        }
    }
    game
}

const EXPORT_FILE: &str = "game.json";

fn import(game: Game) -> Game {
    let file = File::open(EXPORT_FILE).expect("Failed to open export file");
    let reader = BufReader::new(file);
    let data: GameData = serde_json::from_reader(reader).expect("Failed to read export file");
    Game::from_data(data, game.cache, GameContext::Play)
}

fn export(game: &Game) {
    serde_json::to_writer_pretty(
        File::create(EXPORT_FILE).expect("Failed to create export file"),
        &game.cloned_data(),
    )
    .expect("Failed to write export file");
}
