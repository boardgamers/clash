use macroquad::ui::Ui;
use server::game::Game;
use crate::client_state::ShownPlayer;

pub fn show_wonders(game: &Game, player: &ShownPlayer, ui: &mut Ui) {
    //todo move to cards ui
    // let player = game.get_player(player.index);
    // let y = 5.;
    //
    // for (i, card) in player.wonder_cards.iter().enumerate() {
    //     let req = match card.required_advances[..] {
    //         [] => String::from("no advances"),
    //         _ => card.required_advances.join(", "),
    //     };
    //     ui.label(
    //         vec2(900. + i as f32 * 30.0, y),
    //         &format!(
    //             "Wonder Card {} cost {} requires {}",
    //             &card.name, card.cost, req
    //         ),
    //     );
    // }
}
