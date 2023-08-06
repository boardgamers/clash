use macroquad::color::BLACK;
use macroquad::text::draw_text;
use server::game::Game;
use server::game::LogItem::{CulturalInfluenceResolutionAction, PlayingAction, StatusPhaseAction};

pub fn show_log(game: &Game) {
    game.log.iter().enumerate().for_each(|(i, l)| {
        let s = match l {
            PlayingAction(a) => a,
            StatusPhaseAction(a) => a,
            CulturalInfluenceResolutionAction(a) => a,
        };
        draw_text(s, 800., 150. + i as f32 * 20., 20., BLACK);
    });
}
