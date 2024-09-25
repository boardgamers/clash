use client::client::Features;
use client::local_client;
use server::game::Game;

#[macroquad::main("Clash")]
async fn main() {
    let wasm = cfg!(feature = "wasm");

    let features = Features {
        import_export: !wasm,
        local_assets: !wasm,
    };

    //todo add button to decide random or fixed game
    let game = if false {
        Game::new(2, "a".repeat(32), true)
    } else {
        local_client::setup_local_game()
    };

    local_client::run(game, &features).await;
}
