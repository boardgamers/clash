#![allow(clippy::missing_panics_doc)]

use client::client::{init, render_and_update, Features, GameSyncRequest, GameSyncResult};
use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::{next_frame, screen_width, vec2};
use macroquad::window::screen_height;
use server::city::City;
use server::game::{Game, GameData};
use server::map::Terrain;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::{UnitType, Units};
use std::env;
use std::fs::File;
use std::io::BufReader;

#[macroquad::main("Clash")]
async fn main() {
    set_window_size(900, 400);

    let features = Features {
        import_export: true,
        assets_url: "assets/".to_string(),
    };

    let args: Vec<String> = env::args().collect();
    let game = if args.len() > 1 && args[1] == "generate" {
        let seed = if args.len() > 2 {
            args[2].parse().unwrap()
        } else {
            "a".repeat(32)
        };
        Game::new(2, seed, true)
    } else {
        setup_local_game()
    };

    run(game, &features).await;
}

pub async fn run(mut game: Game, features: &Features) {
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
            GameSyncRequest::ExecuteAction(a) => {
                game.execute_action(a, game.active_player());
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

#[must_use]
pub fn setup_local_game() -> Game {
    let mut game = Game::new(2, "0".to_string(), false);
    game.round = 6;
    game.dice_roll_outcomes = vec![1, 1, 10, 10, 10, 10, 10, 10, 10, 10];
    let add_unit = |game: &mut Game, pos: &str, player_index: usize, unit_type: UnitType| {
        let mut units = Units::empty();
        units += &unit_type;
        game.recruit(player_index, units, Position::from_offset(pos), None, &[]);
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
        .unwrap()
        .increase_mood_state();
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .unwrap()
        .pieces
        .academy = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .unwrap()
        .pieces
        .port = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .unwrap()
        .port_position = Some(Position::from_offset("C3"));
    // game.players[player_index1]
    //     .get_city_mut(Position::from_offset("C2"))
    //     .unwrap()
    //     .pieces
    //     .wonders = vec![game.wonders_left.pop().unwrap()];
    let gardens = game.wonders_left.remove(
        game.wonders_left
            .iter()
            .position(|w| w.name == "Great Gardens")
            .unwrap(),
    );
    game.players[player_index1].wonder_cards.push(gardens);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("C2"))
        .unwrap()
        .increase_mood_state();

    game.players[player_index2]
        .get_city_mut(Position::from_offset("B2"))
        .unwrap()
        .pieces
        .port = Some(1);
    game.players[player_index2]
        .get_city_mut(Position::from_offset("B2"))
        .unwrap()
        .port_position = Some(Position::from_offset("C3"));

    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .unwrap()
        .pieces
        .obelisk = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .unwrap()
        .pieces
        .observatory = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .unwrap()
        .pieces
        .temple = Some(1);
    game.players[player_index1]
        .get_city_mut(Position::from_offset("B1"))
        .unwrap()
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
        .unwrap()
        .carrier_id = Some(ship_id);
    game.players[player_index1]
        .get_unit_mut(elephant)
        .unwrap()
        .position = Position::from_offset("C3");
    game.players[player_index1]
        .get_unit_mut(cavalry)
        .unwrap()
        .carrier_id = Some(ship_id);
    game.players[player_index1]
        .get_unit_mut(cavalry)
        .unwrap()
        .position = Position::from_offset("C3");

    game.players[player_index1]
        .get_city_mut(Position::from_offset("A1"))
        .unwrap()
        .pieces
        .market = Some(1);

    game.advance("Voting", player_index1, ResourcePile::empty());
    game.advance("Free Economy", player_index1, ResourcePile::empty());
    game.advance("Storage", player_index1, ResourcePile::empty());
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
