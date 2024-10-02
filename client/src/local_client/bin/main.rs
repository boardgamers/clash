use client::client::Features;
use client::local_client;
use macroquad::window::set_fullscreen;
use server::game::Game;

#[macroquad::main("Clash")]
async fn main() {
    set_fullscreen(true);

    let features = Features {
        import_export: false,
        assets_url: "assets/".to_string(),
    };

    //todo add button to decide random or fixed game
    let game = if false {
        Game::new(2, "a".repeat(32), true)
    } else {
        local_client::setup_local_game()
    };

    local_client::run(game, &features).await;
}
