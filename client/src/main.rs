mod map;
mod ui;

use crate::map::pixel_to_coordinate;
use macroquad::prelude::*;
use server::city::City;
use server::game::Game;
use server::hexagon::Position;

#[macroquad::main("Clash")]
async fn main() {
    let mut status: String = "".to_string();

    let mut game = Game::new(1, "a".repeat(32));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("A4")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("A1")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("A2")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("A3")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("B4")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("B1")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("B2")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("B3")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("C4")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("C1")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("C2")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("C3")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("D4")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("D1")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("D2")));
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("D3")));

    loop {
        clear_background(RED);

        for p in game.players.iter() {
            for city in p.cities.iter() {
                map::draw_hex(&city.position);
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();

            let c = pixel_to_coordinate(x, y);

            status = "".to_string();

            for p in game.players.iter() {
                for city in p.cities.iter() {
                    let pos = &city.position;
                    if c == pos.coordinate() {
                        let n = pos.to_string();
                        status = format!("clicked city {n}")
                    };
                }
            }
        }
        draw_text(&status, 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }
}
