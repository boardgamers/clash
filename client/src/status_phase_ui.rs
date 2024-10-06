use crate::client_state::{ActiveDialog, ShownPlayer, StateUpdate};
use crate::dialog_ui::active_dialog_window;
use crate::select_ui;
use crate::select_ui::{ConfirmSelection, Selection, SelectionConfirm};
use server::action::Action;
use server::content::advances;
use server::game::Game;
use server::position::Position;
use server::status_phase::{ChangeGovernmentType, StatusPhaseAction};

pub fn determine_first_player_dialog(game: &Game, player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(
        player,
        "Who should be the first player in the next age?",
        |ui| {
            for p in &game.players {
                if ui.button(
                    None,
                    format!("Player {} - {}", p.index, p.civilization.name),
                ) {
                    return StateUpdate::status_phase(StatusPhaseAction::DetermineFirstPlayer(
                        p.index,
                    ));
                }
            }
            StateUpdate::None
        },
    )
}

pub fn raze_city_confirm_dialog(game: &Game, player: &ShownPlayer, pos: Position) -> StateUpdate {
    if player.get(game).can_raze_city(pos) {
        StateUpdate::execute_with_confirm(
            vec![format!("Raze {pos} to get 1 gold")],
            Action::StatusPhase(StatusPhaseAction::RaseSize1City(Some(pos))),
        )
    } else {
        StateUpdate::None
    }
}

pub fn raze_city_dialog(player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(player, "Select a city to raze - or decline.", |ui| {
        if ui.button(None, "Decline") {
            return StateUpdate::status_phase(StatusPhaseAction::RaseSize1City(None));
        }
        StateUpdate::None
    })
}

#[derive(Clone)]
pub struct ChooseAdditionalAdvances {
    government: String,
    advances: Vec<String>,
    selected: Vec<String>,
}

impl ChooseAdditionalAdvances {
    fn new(government: String, advances: Vec<String>) -> Self {
        Self {
            government,
            advances,
            selected: Vec::new(),
        }
    }
}

impl Selection for ChooseAdditionalAdvances {
    fn all(&self) -> &[String] {
        &self.advances
    }

    fn selected(&self) -> &[String] {
        &self.selected
    }

    fn selected_mut(&mut self) -> &mut Vec<String> {
        &mut self.selected
    }

    fn can_select(&self, _game: &Game, _name: &str) -> bool {
        true
    }
}

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

pub fn change_government_type_dialog(game: &Game, player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(player, "Select additional advances", |ui| {
        let current = player
            .get(game)
            .government()
            .expect("should have government");
        for (g, _) in advances::get_governments()
            .iter()
            .filter(|(g, _)| g != &current)
        {
            if ui.button(None, format!("Change to {g}")) {
                let additional = advances::get_government(g)
                    .iter()
                    .skip(1) // the government advance itself is always chosen
                    .map(|a| a.name.clone())
                    .collect::<Vec<_>>();
                return StateUpdate::SetDialog(ActiveDialog::ChooseAdditionalAdvances(
                    ChooseAdditionalAdvances::new(g.clone(), additional),
                ));
            }
        }

        if ui.button(None, "Decline") {
            return StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(None));
        }
        StateUpdate::None
    })
}

pub fn choose_additional_advances_dialog(
    game: &Game,
    additional_advances: &ChooseAdditionalAdvances,
    player: &ShownPlayer,
) -> StateUpdate {
    select_ui::selection_dialog(
        game,
        player,
        "Select additional advances:",
        additional_advances,
        |a| StateUpdate::SetDialog(ActiveDialog::ChooseAdditionalAdvances(a)),
        |a| {
            StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(Some(
                ChangeGovernmentType {
                    new_government: a.government.clone(),
                    additional_advances: a.selected.clone(),
                },
            )))
        },
    )
}

pub fn complete_objectives_dialog(player: &ShownPlayer) -> StateUpdate {
    active_dialog_window(player, "Complete Objectives", |ui| {
        if ui.button(None, "None") {
            return StateUpdate::status_phase(StatusPhaseAction::CompleteObjectives(vec![]));
        }
        StateUpdate::None
    })
}
