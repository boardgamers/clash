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

mod advance_ui;
mod city_ui;
mod collect_ui;
mod construct_ui;
mod dialog_ui;
mod game_loop;
mod happiness_ui;
mod hex_ui;
mod influence_ui;
mod local_ui;
mod log_ui;
mod map_ui;
mod payment_ui;
mod player_ui;
mod resource_ui;
mod ui_state;

#[macroquad::main("Clash")]
async fn main() {
    let mut game = local_ui::setup_local_game();

    game_loop::run(&mut game).await;
}
