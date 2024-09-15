use macroquad::prelude::*;
use server::game::Game;

#[macroquad::main("Clash")]
async fn main() {
    loop {
        clear_background(RED);

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("IT WORKS! 3", 20.0, 20.0, 30.0, DARKGRAY);

        next_frame().await
    }

    // //todo add button to decide random or fixed game
    //   let game = if true {
    //       Game::new(2, "c".repeat(32), true)
    //   } else {
    //       local_client::setup_local_game()
    //   };
    //
    //   local_client::run(game).await;
}
