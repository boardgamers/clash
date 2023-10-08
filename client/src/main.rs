#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

extern crate core;

use server::game::Game;

use crate::client_state::ActiveDialog;

mod advance_ui;
mod assets;
mod city_ui;
mod client_state;
mod collect_ui;
mod combat_ui;
mod construct_ui;
mod dialog_ui;
mod game_loop;
mod game_sync;
mod happiness_ui;
mod hex_ui;
mod influence_ui;
mod local_client;
mod log_ui;
mod map_ui;
mod move_ui;
mod payment_ui;
mod player_ui;
mod recruit_unit_ui;
mod remote_client;
mod resource_ui;
mod select_ui;
mod status_phase_ui;
mod unit_ui;

#[macroquad::main("Clash")]
async fn main() {
    //todo add button to decide random or fixed game
    let game = if true {
        Game::new(2, "c".repeat(32), true)
    } else {
        local_client::setup_local_game()
    };

    local_client::run(game).await;
}
