#![allow(clippy::missing_panics_doc)]

use client::client::{Features, GameSyncRequest, GameSyncResult, init, render_and_update};
use client::client_state::State;
use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::{next_frame, screen_width, vec2};
use macroquad::window::screen_height;
use pyroscope::PyroscopeAgent;
use pyroscope_pprofrs::{PprofConfig, pprof_backend};
use server::action::execute_action;
use server::advance::do_advance;
use server::ai::AI;
use server::city::City;
use server::content::advances::get_advance;
use server::game::{Game, GameData};
use server::game_setup::setup_game;
use server::map::Terrain;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::UnitType;
use server::utils::remove_element;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use std::{env, vec};

#[derive(PartialEq)]
enum Mode {
    Local,
    AI,
    Profile,
    Test,
}

#[macroquad::main("Clash")]
async fn main() {
    set_window_size(1200, 600);

    let args: Vec<String> = env::args().collect();
    let modes = get_modes(&args);

    if modes.contains(&Mode::Profile) {
        let pprof_config = PprofConfig::new().sample_rate(100);
        let backend_impl = pprof_backend(pprof_config);

        let agent = PyroscopeAgent::builder("http://localhost:4040", "clash")
            .backend(backend_impl)
            .build()
            .expect("Failed to initialize pyroscope");
        let _ = agent.start().unwrap();
    }

    let mut features = Features {
        import_export: true,
        assets_url: "assets/".to_string(),
        ai: modes
            .contains(&Mode::AI)
            .then(|| AI::new(1., Duration::from_secs(5), false)),
    };

    let game = if modes.contains(&Mode::Test) {
        setup_local_game()
    } else {
        let seed = if args.len() > 2 {
            args[2].parse().unwrap()
        } else {
            "a".repeat(32)
        };
        setup_game(2, seed, true)
    };

    run(game, &mut features).await;
}

fn get_modes(args: &[String]) -> Vec<Mode> {
    match args.get(1) {
        Some(arg) => match arg.as_str() {
            "generate" => vec![Mode::Local],
            "ai" => vec![Mode::AI],
            "profile" => vec![Mode::AI, Mode::Profile],
            _ => {
                panic!("Unknown argument: {}", arg);
            }
        },
        _ => vec![Mode::Test],
    }
}

async fn run(mut game: Game, features: &mut Features) {
    let mut state = init(features).await;

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
                game = import();
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

fn ai_autoplay(game: Game, f: &mut Features, state: &mut State) -> Game {
    if let Some(ai) = &mut f.ai {
        if state.ai_autoplay {
            // todo does this block the ui?
            // state.ai_autoplay = false;
            let action = ai.next_action(&game);
            let player_index = game.active_player();
            return execute_action(game, action, player_index);
        }
    }
    game
}

#[must_use]
fn setup_local_game() -> Game {
    let mut game = setup_game(2, "0".to_string(), false);
    game.round = 6;
    game.dice_roll_outcomes = vec![1, 1, 10, 10, 10, 10, 10, 10, 10, 10];
    let add_unit = |game: &mut Game, pos: &str, player_index: usize, unit_type: UnitType| {
        game.player_mut(player_index)
            .add_unit(Position::from_offset(pos), unit_type);
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
    // game.players[player_index1].active_leader =
    //     Some(Leader::builder("Alexander", "", "", "", "").build());

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
        .push(remove_element(&mut game.wonders_left, &"Great Gardens".to_string()).unwrap());
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
    game.players[player_index1].get_unit_mut(elephant).position = Position::from_offset("C3");
    game.players[player_index1].get_unit_mut(cavalry).carrier_id = Some(ship_id);
    game.players[player_index1].get_unit_mut(cavalry).position = Position::from_offset("C3");

    game.players[player_index1]
        .get_city_mut(Position::from_offset("A1"))
        .pieces
        .market = Some(1);

    do_advance(&mut game, &get_advance("Voting"), player_index1);
    do_advance(&mut game, &get_advance("Free Economy"), player_index1);
    do_advance(&mut game, &get_advance("Storage"), player_index1);
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

fn import() -> Game {
    let file = File::open(EXPORT_FILE).expect("Failed to open export file");
    let reader = BufReader::new(file);
    let data: GameData = serde_json::from_reader(reader).expect("Failed to read export file");
    Game::from_data(data)
}

fn export(game: &Game) {
    serde_json::to_writer_pretty(
        File::create(EXPORT_FILE).expect("Failed to create export file"),
        &game.cloned_data(),
    )
    .expect("Failed to write export file");
}
