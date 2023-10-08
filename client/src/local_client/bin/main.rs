use server::game::Game;

use client::local_client;

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
