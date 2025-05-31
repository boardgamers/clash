#![allow(clippy::missing_panics_doc)]

use client::client::{Features, GameSyncRequest, GameSyncResult, init, render_and_update};
use client::client_state::State;
use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::{next_frame, screen_width, vec2};
use macroquad::window::screen_height;
use server::action::execute_action;
use server::advance::{Advance, do_advance};
use server::city::City;
use server::game::{Game, GameContext, GameOptions, UndoOption};
use server::game_data::GameData;
use server::game_setup::{GameSetupBuilder, setup_game};
use server::map::Terrain;
use server::player::gain_unit;
use server::position::Position;
use server::profiling::start_profiling;
use server::resource_pile::ResourcePile;
use server::unit::{UnitType, set_unit_position};
use server::utils::remove_element;
use server::wonder::Wonder;
use std::fs::File;
use std::io::BufReader;
use std::{env, vec};

#[derive(PartialEq)]
enum Mode {
    Local,
    AI,
    Test,
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

    let game = if modes.contains(&Mode::Test) {
        setup_local_game()
    } else {
        let seed = if args.len() > 2 {
            args[2].parse().unwrap()
        } else {
            "a".repeat(32)
        };
        setup_game(
            GameSetupBuilder::new(players)
                .seed(seed)
                .options(GameOptions {
                    undo: UndoOption::SamePlayer,
                })
                .build(),
        )
    };

    run(game, &mut features).await;
}

fn get_modes(args: &[String]) -> Vec<Mode> {
    match args.first() {
        Some(arg) => match arg.as_str() {
            "generate" => vec![Mode::Local],
            "ai" => vec![Mode::AI, Mode::Local],
            _ => {
                panic!("Unknown argument: {}", arg);
            }
        },
        _ => vec![Mode::Test],
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

#[must_use]
fn setup_local_game() -> Game {
    let mut game = setup_game(GameSetupBuilder::new(2).skip_random_map().build());
    game.round = 1;
    game.dice_roll_outcomes = vec![1, 1, 10, 10, 10, 10, 10, 10, 10, 10];
    let add_unit = |game: &mut Game, pos: &str, player_index: usize, unit_type: UnitType| {
        gain_unit(player_index, Position::from_offset(pos), unit_type, game);
    };

    let player_index1 = 0;
    let player_index2 = 1;
    game.players[player_index1].gain_resources(ResourcePile::new(0, 5, 5, 5, 5, 9, 9));
    game.players[player_index2].gain_resources(ResourcePile::new(0, 5, 5, 5, 5, 9, 9));
    add_city(&mut game, player_index1, "A1");
    add_city(&mut game, player_index1, "C2");
    add_city(&mut game, player_index1, "B1");
    add_city(&mut game, player_index1, "B3");
    add_city(&mut game, player_index2, "C1");
    add_city(&mut game, player_index2, "B2");

    add_terrain(&mut game, "A1", Terrain::Fertile);
    add_terrain(&mut game, "A2", Terrain::Water);
    add_terrain(
        &mut game,
        "A3",
        Terrain::Exhausted(Box::new(Terrain::Forest)),
    );
    add_terrain(&mut game, "A4", Terrain::Mountain);
    add_terrain(&mut game, "B1", Terrain::Mountain);
    add_terrain(&mut game, "B2", Terrain::Forest);
    add_terrain(&mut game, "B3", Terrain::Fertile);
    add_terrain(&mut game, "B4", Terrain::Fertile);
    add_terrain(&mut game, "C1", Terrain::Barren);
    add_terrain(&mut game, "C2", Terrain::Forest);
    add_terrain(&mut game, "C3", Terrain::Water);
    add_terrain(&mut game, "C4", Terrain::Water);
    add_terrain(&mut game, "C5", Terrain::Water);
    add_terrain(&mut game, "D1", Terrain::Fertile);
    add_terrain(&mut game, "E2", Terrain::Fertile);
    add_terrain(&mut game, "B5", Terrain::Fertile);
    add_terrain(&mut game, "B6", Terrain::Fertile);
    add_terrain(&mut game, "D2", Terrain::Water);

    add_unit(&mut game, "C2", player_index1, UnitType::Infantry);
    add_unit(&mut game, "C2", player_index1, UnitType::Cavalry);
    // add_unit(&mut game, "C2", player_index1, UnitType::Leader);
    add_unit(&mut game, "C2", player_index1, UnitType::Elephant);
    add_unit(&mut game, "B3", player_index1, UnitType::Settler);
    add_unit(&mut game, "B3", player_index1, UnitType::Settler);
    add_unit(&mut game, "B3", player_index1, UnitType::Settler);
    add_unit(&mut game, "B3", player_index1, UnitType::Settler);

    add_unit(&mut game, "C1", player_index2, UnitType::Infantry);
    add_unit(&mut game, "C1", player_index2, UnitType::Infantry);

    game.players[player_index1]
        .get_city_mut(Position::from_offset("A1"))
        .increase_mood_state();
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .pieces
        .academy = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .pieces
        .port = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .port_position = Some(Position::from_offset("C3"));
    // game.players[player_index1]
    //     .get_city_mut(Position::from_offset("C2"))
    //     .unwrap()
    //     .pieces
    //     .wonders = vec![game.wonders_left.pop().unwrap()];

    game.players[player_index1]
        .wonder_cards
        .push(remove_element(&mut game.wonders_left, &Wonder::GreatGardens).unwrap());
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .increase_mood_state();

    game.players[player_index2]
        .get_city_mut(Position::from_offset("B2"))
        .pieces
        .port = Some(1);
    game.players[player_index2]
        .get_city_mut(Position::from_offset("B2"))
        .port_position = Some(Position::from_offset("C3"));

    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .pieces
        .obelisk = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .pieces
        .observatory = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .pieces
        .temple = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .pieces
        .fortress = Some(1);

    add_unit(&mut game, "C2", player_index1, UnitType::Ship);
    add_unit(&mut game, "C2", player_index1, UnitType::Ship);
    add_unit(&mut game, "C2", player_index1, UnitType::Ship);

    let ship_id = game.players[player_index1]
        .units
        .iter()
        .find(|u| u.unit_type == UnitType::Ship)
        .map(|u| u.id)
        .unwrap();
    let elephant = game.players[player_index1]
        .units
        .iter()
        .find(|u| u.unit_type == UnitType::Elephant)
        .map(|u| u.id)
        .unwrap();
    let cavalry = game.players[player_index1]
        .units
        .iter()
        .find(|u| u.unit_type == UnitType::Cavalry)
        .map(|u| u.id)
        .unwrap();

    game.players[player_index1]
        .get_unit_mut(elephant)
        .carrier_id = Some(ship_id);
    set_unit_position(
        player_index1,
        elephant,
        Position::from_offset("C3"),
        &mut game,
    );
    game.players[player_index1].get_unit_mut(cavalry).carrier_id = Some(ship_id);
    set_unit_position(
        player_index1,
        cavalry,
        Position::from_offset("C3"),
        &mut game,
    );

    game.players[player_index1]
        .get_city_mut(Position::from_offset("A1"))
        .pieces
        .market = Some(1);

    do_advance(&mut game, Advance::Voting, player_index1);
    do_advance(&mut game, Advance::FreeEconomy, player_index1);
    do_advance(&mut game, Advance::Storage, player_index1);
    game.players[player_index1].gain_resources(ResourcePile::food(5));

    game
}

fn add_city(game: &mut Game, player_index: usize, s: &str) {
    game.players[player_index]
        .cities
        .push(City::new(player_index, Position::from_offset(s)));
}

fn add_terrain(game: &mut Game, pos: &str, terrain: Terrain) {
    game.map.tiles.insert(Position::from_offset(pos), terrain);
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
