use server::action::{Action, CombatAction};

use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{StateUpdate, StateUpdates};

pub fn retreat_dialog() -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        ui.label(None, "Do you want to retreat?");
        if ui.button(None, "Retreat") {
            updates.add(retreat(true));
        }
        if ui.button(None, "Decline") {
            updates.add(retreat(false));
        }
    });
    updates.result()
}

fn retreat(retreat: bool) -> StateUpdate {
    StateUpdate::Execute(Action::Combat(CombatAction::Retreat(retreat)))
}

pub fn place_settler_dialog() -> StateUpdate {
    active_dialog_window(|ui| {
        ui.label(None, "Select a city to place a settler in.");
    });
    StateUpdate::None
}
