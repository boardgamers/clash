extern crate core;

use macroquad::prelude::*;
use server::city::City;
use server::game::Game;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::advance_ui::{pay_advance_dialog, show_advance_menu};
use crate::city_ui::{pay_construction_dialog, show_city_menu, try_city_click};
use crate::log_ui::show_log;
use crate::map_ui::draw_map;
use crate::player_ui::{
    show_global_controls, show_globals, show_increase_happiness, show_resources,
};
use crate::ui::{ActiveDialog, State};

mod advance_ui;
mod city_ui;
mod log_ui;
mod map_ui;
mod payment_ui;
mod player_ui;
mod ui;

#[macroquad::main("Clash")]
async fn main() {
    let mut game = Game::new(1, "a".repeat(32));
    let player_index = 0;
    let city = City::new(player_index, Position::from_offset("A1"));
    let player = &mut game.players[player_index];
    player.gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    player.cities.push(city);
    player
        .cities
        .push(City::new(player_index, Position::from_offset("C2")));
    player
        .cities
        .push(City::new(player_index, Position::from_offset("C1")));

    let mut state = State::new();

    set_fullscreen(true);
    loop {
        clear_background(GREEN);

        draw_map(&game, &state);
        show_advance_menu(&game, player_index, &mut state);
        show_globals(&game);
        show_log(&game);
        show_resources(&game, player_index);
        show_increase_happiness(&mut game, player_index, &mut state);
        show_global_controls(&mut game, player_index);

        if let Some((player_index, city_position)) = &state.focused_city {
            let dialog = show_city_menu(&game, *player_index, city_position);
            if let Some(dialog) = dialog {
                state.active_dialog = dialog;
            }
        }

        match &mut state.active_dialog {
            ActiveDialog::AdvancePayment(p) => {
                if pay_advance_dialog(&mut game, p) {
                    state.active_dialog = ActiveDialog::None;
                }
            }
            ActiveDialog::ConstructionPayment(p) => {
                if pay_construction_dialog(&mut game, p) {
                    state.active_dialog = ActiveDialog::None;
                }
            }
            ActiveDialog::None => {}
        }

        try_city_click(&game, &mut state);

        next_frame().await
    }
}
