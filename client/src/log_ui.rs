use macroquad::color::BLACK;
use macroquad::text::draw_text;
use server::game::Game;

pub fn show_log(game: &Game) {
    game.log.iter().enumerate().for_each(|(i, l)| {
        draw_text(l, 1500., 150. + i as f32 * 20., 20., BLACK);
    });
}
