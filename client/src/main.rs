extern crate core;



use crate::ui::{ActiveDialog, State};

mod advance_ui;
mod city_ui;
mod log_ui;
mod map_ui;
mod payment_ui;
mod player_ui;
mod ui;
mod local_ui;
mod game_loop;

#[macroquad::main("Clash")]
async fn main() {
    let mut game = local_ui::setup_local_game();

    game_loop::run(&mut game).await;
}
