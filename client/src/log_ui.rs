use server::game::Game;

use crate::dialog_ui::closeable_dialog_window;
use crate::ui_state::StateUpdate;

pub fn show_log(game: &Game) -> StateUpdate {
    closeable_dialog_window("Log", |ui| {
        game.log.iter().for_each(|l| {
            ui.label(None, l);
        });

        StateUpdate::None
    })
}
