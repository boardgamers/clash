use server::game::Game;

use crate::dialog_ui::dialog_window;
use crate::ui_state::StateUpdate;

pub fn show_log(game: &Game) -> StateUpdate {
    dialog_window(true, |ui| {
        game.log.iter().for_each(|l| {
            ui.label(None, l);
        });

        StateUpdate::None
    })
}
