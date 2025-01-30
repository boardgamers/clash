use crate::client_state::StateUpdate;
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::render_context::RenderContext;
use crate::select_ui::ConfirmSelection;
use crate::unit_ui;
use crate::unit_ui::UnitSelection;
use server::action::{Action, CombatAction, PlayActionCard};
use server::game::Game;
use server::position::Position;
use server::unit::Unit;

pub fn retreat_dialog(rc: &RenderContext) -> StateUpdate {
    if ok_button(rc, OkTooltip::Valid("Retreat".to_string())) {
        return retreat(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
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

    fn confirm(&self, _game: &Game) -> OkTooltip {
        if self.needed == self.units.len() as u8 {
            OkTooltip::Valid("Remove casualties".to_string())
        } else {
            OkTooltip::Invalid(format!(
                "Need to select {} units",
                self.needed - self.units.len() as u8
            ))
        }
    }
}

pub fn remove_casualties_dialog(
    rc: &RenderContext,
    sel: &RemoveCasualtiesSelection,
) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RemoveCasualtiesSelection>(
        rc,
        sel,
        |new: RemoveCasualtiesSelection| {
            StateUpdate::Execute(Action::Combat(CombatAction::RemoveCasualties(
                new.units.clone(),
            )))
        },
    )
}

pub fn play_action_card_dialog(rc: &RenderContext) -> StateUpdate {
    if cancel_button_with_tooltip(rc, "Play no action card") {
        return StateUpdate::Execute(Action::Combat(CombatAction::PlayActionCard(
            PlayActionCard::None,
        )));
    }
    StateUpdate::None
}

