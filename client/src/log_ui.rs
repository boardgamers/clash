use macroquad::ui::Ui;
use server::game::Game;

use crate::client_state::StateUpdate;
use crate::dialog_ui::closeable_dialog_window;

pub fn show_log(game: &Game) -> StateUpdate {
    closeable_dialog_window("Log", |ui| {
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
        line.push_str(s);
    });
    if !line.is_empty() {
        ui.label(None, &line);
    }
}
