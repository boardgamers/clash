use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{StateUpdate, StateUpdates};
use server::game::Game;
use server::status_phase::StatusPhaseAction;

pub fn determine_first_player_dialog(game: &Game) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        ui.label(None, "Who should be the first player in the next age?");
        game.players.iter().for_each(|p| {
            if ui.button(
                None,
                format!("Player {} - {}", p.index, p.civilization.name),
            ) {
                updates.add(StateUpdate::status_phase(
                    StatusPhaseAction::DetermineFirstPlayer(p.index),
                ));
            }
        });
    });
    updates.result()
}

pub fn raze_city_dialog() -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        ui.label(None, "Select a city to raze - or decline.");
        if ui.button(None, "Decline") {
            updates.add(StateUpdate::status_phase(StatusPhaseAction::RaseSize1City(
                None,
            )));
        }
    });
    updates.result()
}
