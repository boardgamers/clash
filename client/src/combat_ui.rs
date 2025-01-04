use server::action::{Action, CombatAction, PlayActionCard};
use server::game::Game;
use server::position::Position;
use server::unit::Unit;

use crate::client_state::{State, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button_with_tooltip};
use crate::select_ui::{ConfirmSelection, SelectionConfirm};
use crate::unit_ui;
use crate::unit_ui::UnitSelection;

pub fn retreat_dialog(state: &State) -> StateUpdate {
    if ok_button_with_tooltip(state, true, "Retreat") {
        return retreat(true);
    }
    if cancel_button_with_tooltip(state, "Decline") {
        return retreat(false);
    }
    StateUpdate::None
}

fn retreat(retreat: bool) -> StateUpdate {
    StateUpdate::Execute(Action::Combat(CombatAction::Retreat(retreat)))
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
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selectable.contains(&unit.id)
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

pub fn remove_casualties_dialog(
    game: &Game,
    sel: &RemoveCasualtiesSelection,
    state: &State,
) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RemoveCasualtiesSelection>(
        game,
        sel,
        |new: RemoveCasualtiesSelection| {
            StateUpdate::Execute(Action::Combat(CombatAction::RemoveCasualties(
                new.units.clone(),
            )))
        },
        state,
    )
}

pub fn play_action_card_dialog(state: &State) -> StateUpdate {
    if cancel_button_with_tooltip(state, "Play no action card") {
        return StateUpdate::Execute(Action::Combat(CombatAction::PlayActionCard(
            PlayActionCard::None,
        )));
    }
    StateUpdate::None
}
