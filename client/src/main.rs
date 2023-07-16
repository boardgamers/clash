extern crate core;

use macroquad::prelude::*;
use server::city::City;
use server::game::Game;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::map::pixel_to_coordinate;
use advance_ui::AdvancePayment;

mod map;
mod payment;
mod ui;
mod advance_ui;
mod city_ui;

pub enum ActiveDialog {
    None,
    AdvancePayment(AdvancePayment),
}

pub struct State {
    pub focused_city: Option<(usize, Position)>,
    pub active_dialog: ActiveDialog,
}

#[macroquad::main("Clash")]
async fn main() {
    let mut game = Game::new(1, "a".repeat(32));
    let city = City::new(0, Position::from_offset("A1"));
    let player = &mut game.players[0];
    player.gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    player.cities.push(city);
    player
        .cities
        .push(City::new(0, Position::from_offset("C2")));
    player
        .cities
        .push(City::new(0, Position::from_offset("C1")));

    let mut state = State {
        active_dialog: ActiveDialog::None,
        focused_city: None,
    };

    loop {
        clear_background(GREEN);

        draw_map(&game);
        advance_ui::show_advance_menu(&mut game, 0, &mut state);
        show_resources(&game, 0);

        if let Some((player_index, city_position)) = &state.focused_city {
            city_ui::show_city_menu(&mut game, *player_index, city_position);
        }

        match &mut state.active_dialog {
            ActiveDialog::AdvancePayment(p) => {
                if advance_ui::buy_advance_menu(&mut game, p) {
                    state.active_dialog = ActiveDialog::None;
                }
            }
            ActiveDialog::None => {}
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();

            let c = pixel_to_coordinate(x, y);

            state.focused_city = None;

            for p in game.players.iter() {
                for city in p.cities.iter() {
                    let pos = city.position.clone();
                    if c == pos.coordinate() {
                        state.focused_city = Some((p.index, pos));
                    };
                }
            }
        }

        next_frame().await
    }
}

fn draw_map(game: &Game) {
    for p in game.players.iter() {
        for city in p.cities.iter() {
            city_ui::draw_city(p, city);
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
