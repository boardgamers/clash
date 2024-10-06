use macroquad::ui::Ui;
use server::game::Game;

use crate::client_state::{ShownPlayer, StateUpdate};
use crate::dialog_ui::dialog;

pub fn show_log(game: &Game, player: &ShownPlayer) -> StateUpdate {
    dialog(player, "Log", |ui| {
        game.log.iter().for_each(|l| {
            multiline(ui, l);
        });
        StateUpdate::None
    })
}

fn multiline(ui: &mut Ui, text: &str) {
    let mut line = String::new();
    text.split(' ').for_each(|s| {
        if line.len() + s.len() > 100 {
            ui.label(None, &line);
            line = String::new();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(s);
    });
    if !line.is_empty() {
        ui.label(None, &line);
    }
}
