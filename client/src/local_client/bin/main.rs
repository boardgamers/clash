use client::local_client;
use macroquad::prelude::*;
use server::game::Game;

#[macroquad::main("Clash")]
async fn main() {
    //todo add button to decide random or fixed game
    let game = if false {
        Game::new(2, "a".repeat(32), true)
    } else {
        local_client::setup_local_game()
    };

    local_client::run(game).await;
}
