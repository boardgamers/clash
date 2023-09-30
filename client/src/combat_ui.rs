use server::action::{Action, CombatAction};
use server::game::Game;
use server::position::Position;
use server::unit::Unit;

use crate::dialog_ui::active_dialog_window;
use crate::select_ui::{ConfirmSelection, SelectionConfirm};
use crate::ui_state::{ActiveDialog, StateUpdate, StateUpdates};
use crate::unit_ui;
use crate::unit_ui::UnitSelection;

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

#[derive(Clone)]
pub struct RemoveCasualtiesSelection {
    pub position: Position,
    pub needed: u8,
    pub selectable: Vec<u32>,
    pub units: Vec<u32>,
}

impl RemoveCasualtiesSelection {
    pub fn new(position: Position, needed: u8, selectable: Vec<u32>) -> Self {
        RemoveCasualtiesSelection {
            position,
            needed,
            units: Vec::new(),
            selectable,
        }
    }
}

impl UnitSelection for RemoveCasualtiesSelection {
    fn selected_units(&self) -> &[u32] {
        &self.units
    }

    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selectable.contains(&unit.id)
    }

    fn current_tile(&self) -> Option<Position> {
        Some(self.position)
    }
}

impl ConfirmSelection for RemoveCasualtiesSelection {
    fn cancel_name(&self) -> Option<&str> {
        None
    }

    fn confirm(&self, _game: &Game) -> SelectionConfirm {
        if self.needed == self.units.len() as u8 {
            SelectionConfirm::Valid
        } else {
            SelectionConfirm::Invalid
        }
    }
}

pub fn remove_casualties_dialog(game: &Game, sel: &RemoveCasualtiesSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RemoveCasualtiesSelection>(
        game,
        sel,
        |new| StateUpdate::SetDialog(ActiveDialog::RemoveCasualties(new.clone())),
        |new: RemoveCasualtiesSelection| {
            StateUpdate::Execute(Action::Combat(CombatAction::RemoveCasualties(
                new.units.clone(),
            )))
        },
    )
}
