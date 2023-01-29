extern crate core;

use macroquad::hash;
use macroquad::prelude::*;
use macroquad::ui::root_ui;

use server::city::{Building, City};
use server::game::Game;
use server::hexagon::Position;
use server::resource_pile::ResourcePile;
use strum::IntoEnumIterator;

use crate::map::pixel_to_coordinate;

mod map;
mod ui;

#[macroquad::main("Clash")]
async fn main() {
    let mut game = Game::new(1, "a".repeat(32));
    let position = Position::from_offset("A1");
    let city = City::new(0, position);
    game.players[0].gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    game.players[0].cities.push(city);
    game.players[0]
        .cities
        .push(City::new(0, Position::from_offset("C2")));

    let mut focused_city: Option<(usize, Position)> = None;

    loop {
        clear_background(RED);

        draw_map(&mut game);
        show_resources(&game, 0);

        if let Some((player_index, city_position)) = focused_city {
            show_city_menu(&mut game, player_index, &city_position);
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();

            let c = pixel_to_coordinate(x, y);

            focused_city = None;

            for p in game.players.iter() {
                for city in p.cities.iter() {
                    let pos = city.position;
                    if c == pos.coordinate() {
                        focused_city = Some((p.index, pos));
                    };
                }
            }
        }

        next_frame().await
    }
}

fn draw_map(game: &mut Game) {
    for p in game.players.iter() {
        for city in p.cities.iter() {
            map::draw_city(p, city);
        }
    }
}

fn show_resources(game: &Game, player_index: usize) {
    let player = &game.players[player_index];
    let r: &ResourcePile = player.resources();

     let mut i: f32 = 0.;
    let mut res = |label: String| {
        draw_text(
            &label,
            600.,
            300. + player_index as f32 * 200. + i,
            20.,
            BLACK,
        );
        i += 30.;
    };

    res(format!("Food {}", r.food));
    res(format!("Wood {}", r.wood));
    res(format!("Ore {}", r.ore));
    res(format!("Ideas {}", r.ideas));
    res(format!("Gold {}", r.gold));
    res(format!("Mood {}", r.mood_tokens));
    res(format!("Culture {}", r.culture_tokens));
}

fn show_city_menu(game: &mut Game, player_index: usize, city_position: &Position) {
    root_ui().window(hash!(), vec2(600., 20.), vec2(100., 200.), |ui| {
        for b in Building::iter() {
            if game.players[player_index]
                .get_city(city_position)
                .expect("city not found")
                .can_increase_size(&b, &game.players[0])
            {
                let string = format!("{b}");
                if ui.button(None, string) {
                    game.players[0].increase_size(&b, city_position);
                }
            }
        }
    });
}
