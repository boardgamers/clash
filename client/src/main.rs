#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

extern crate core;

use crate::ui_state::ActiveDialog;
use server::game::Game;

mod advance_ui;
mod assets;
mod city_ui;
mod collect_ui;
mod combat_ui;
mod construct_ui;
mod dialog_ui;
mod game_loop;
mod happiness_ui;
mod hex_ui;
mod influence_ui;
mod local_ui;
mod log_ui;
mod map_ui;
mod move_ui;
mod payment_ui;
mod player_ui;
mod recruit_unit_ui;
mod resource_ui;
mod select_ui;
mod status_phase_ui;
mod ui_state;
mod unit_ui;

#[macroquad::main("Clash")]
async fn main() {
    //todo add button to decide random or fixed game
    let mut game = if true {
        Game::new(2, "a".repeat(32))
    } else {
        local_ui::setup_local_game()
    };

    game_loop::run(&mut game).await;
}
