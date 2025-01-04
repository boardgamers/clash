use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::dialog_ui::{cancel_button, cancel_button_with_tooltip};
use crate::select_ui::{confirm_update, ConfirmSelection, SelectionConfirm};
use server::action::Action;
use server::game::Game;
use server::position::Position;
use server::status_phase::{
    ChangeGovernment, ChangeGovernmentType, RazeSize1City, StatusPhaseAction,
};

pub fn raze_city_confirm_dialog(game: &Game, player: &ShownPlayer, pos: Position) -> StateUpdate {
    if player.get(game).can_raze_city(pos) {
        StateUpdate::execute_with_confirm(
            vec![format!("Raze {pos} to get 1 gold")],
            Action::StatusPhase(StatusPhaseAction::RazeSize1City(RazeSize1City::Position(
                pos,
            ))),
        )
    } else {
        StateUpdate::None
    }
}

pub fn raze_city_dialog(state: &State) -> StateUpdate {
    if cancel_button(state) {
        return StateUpdate::status_phase(StatusPhaseAction::RazeSize1City(RazeSize1City::None));
    }
    StateUpdate::None
}

#[derive(Clone)]
pub struct ChooseAdditionalAdvances {
    government: String,
    advances: Vec<String>,
    selected: Vec<String>,
}

// impl ChooseAdditionalAdvances {
//     fn new(government: String, advances: Vec<String>) -> Self {
//         Self {
//             government,
//             advances,
//             selected: Vec::new(),
//         }
//     }
// }

impl ConfirmSelection for ChooseAdditionalAdvances {
    fn cancel_name(&self) -> Option<&str> {
        Some("Back to choose government type")
    }

    fn cancel(&self) -> StateUpdate {
        StateUpdate::SetDialog(ActiveDialog::ChangeGovernmentType)
    }

    fn confirm(&self, _game: &Game) -> SelectionConfirm {
        if self.selected.len() == self.advances.len() {
            SelectionConfirm::Valid
        } else {
            SelectionConfirm::Invalid
        }
    }
}

pub fn change_government_type_dialog() -> StateUpdate {
    //todo integrate in advance selection dialog
    // active_dialog_window(player, "Select additional advances", |ui| {
    //     let current = player
    //         .get(game)
    //         .government()
    //         .expect("should have government");
    //     for (g, _) in advances::get_governments()
    //         .iter()
    //         .filter(|(g, _)| g != &current)
    //     {
    //         if ui.button(None, format!("Change to {g}")) {
    //             let additional = advances::get_government(g)
    //                 .iter()
    //                 .skip(1) // the government advance itself is always chosen
    //                 .map(|a| a.name.clone())
    //                 .collect::<Vec<_>>();
    //             return StateUpdate::SetDialog(ActiveDialog::ChooseAdditionalAdvances(
    //                 ChooseAdditionalAdvances::new(g.clone(), additional),
    //             ));
    //         }
    //     }
    //
    //     if ui.button(None, "Decline") {
    //         return StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(
    //             ChangeGovernmentType::KeepGovernment,
    //         ));
    //     }
    //     StateUpdate::None
    // })
    StateUpdate::None
}

pub fn choose_additional_advances_dialog(
    game: &Game,
    a: &ChooseAdditionalAdvances,
    state: &State,
) -> StateUpdate {
    // todo actual selection should be done in advance selection dialog
    confirm_update(
        a,
        || {
            StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(
                ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                    new_government: a.government.clone(),
                    additional_advances: a.selected.clone(),
                }),
            ))
        },
        &a.confirm(game),
        state,
    )
}

pub fn complete_objectives_dialog(state: &State) -> StateUpdate {
    if cancel_button_with_tooltip(state, "Complete no objectives") {
        return StateUpdate::status_phase(StatusPhaseAction::CompleteObjectives(vec![]));
    }
    StateUpdate::None
}
