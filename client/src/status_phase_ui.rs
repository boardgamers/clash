use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button, cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::player_ui::choose_player_dialog;
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::advances::{get_government, get_governments};
use server::position::Position;
use server::status_phase::{
    ChangeGovernment, ChangeGovernmentType, RazeSize1City, StatusPhaseAction,
};

pub fn raze_city_confirm_dialog(rc: &RenderContext, pos: Position) -> StateUpdate {
    if rc.shown_player.can_raze_city(pos) {
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

pub fn raze_city_dialog(rc: &RenderContext) -> StateUpdate {
    if cancel_button(rc) {
        return StateUpdate::status_phase(StatusPhaseAction::RazeSize1City(RazeSize1City::None));
    }
    StateUpdate::None
}

#[derive(Clone)]
pub struct ChooseAdditionalAdvances {
    government: String,
    possible: Vec<String>,
    selected: Vec<String>,
    needed: usize,
}

impl ChooseAdditionalAdvances {
    fn new(government: String, possible: Vec<String>, needed: usize) -> Self {
        Self {
            government,
            possible,
            selected: Vec::new(),
            needed,
        }
    }
}

pub fn change_government_type_dialog(rc: &RenderContext) -> StateUpdate {
    let current = rc.shown_player.government().unwrap();
    if cancel_button_with_tooltip(rc, &format!("Keep {current}")) {
        return StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(
            ChangeGovernmentType::KeepGovernment,
        ));
    }
    show_advance_menu(
        rc,
        "Change government - or click cancel",
        |a, p| {
            if get_governments()
                .iter()
                .find(|g| g.advances[0].name == a.name)
                .is_some_and(|_| p.can_advance_in_change_government(a))
            {
                AdvanceState::Available
            } else if a.government.as_ref().is_some_and(|g| g == &current) {
                AdvanceState::Owned
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            let g = a.government.as_ref().expect("should have government");
            let additional = get_government(g)
                .unwrap()
                .advances
                .iter()
                .skip(1) // the government advance itself is always chosen
                .map(|a| a.name.clone())
                .collect::<Vec<_>>();
            let needed = get_government(&rc.shown_player.government().unwrap())
                .unwrap()
                .advances
                .iter()
                .filter(|a| rc.shown_player.has_advance(&a.name))
                .count()
                - 1;
            StateUpdate::OpenDialog(ActiveDialog::ChooseAdditionalAdvances(
                ChooseAdditionalAdvances::new(g.clone(), additional, needed),
            ))
        },
    )
}

pub fn choose_additional_advances_dialog(
    rc: &RenderContext,
    choose: &ChooseAdditionalAdvances,
) -> StateUpdate {
    let t = if choose.selected.len() == choose.needed {
        OkTooltip::Valid("Change government type".to_string())
    } else {
        OkTooltip::Invalid("Select all additional advances".to_string())
    };
    if ok_button(rc, t) {
        return StateUpdate::status_phase(StatusPhaseAction::ChangeGovernmentType(
            ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                new_government: choose.government.clone(),
                additional_advances: choose.selected.clone(),
            }),
        ));
    }

    if cancel_button_with_tooltip(rc, "Back to choose government type") {
        return StateUpdate::OpenDialog(ActiveDialog::ChangeGovernmentType);
    }
    show_advance_menu(
        rc,
        &format!("Choose additional advances for {}", choose.government),
        |a, _| {
            if choose.selected.contains(&a.name) {
                AdvanceState::Removable
            } else if choose.possible.contains(&a.name) && choose.selected.len() < choose.needed {
                AdvanceState::Available
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            let mut selected = choose.selected.clone();
            if selected.contains(&a.name) {
                selected.retain(|n| n != &a.name);
            } else {
                selected.push(a.name.clone());
            }
            StateUpdate::OpenDialog(ActiveDialog::ChooseAdditionalAdvances(
                ChooseAdditionalAdvances {
                    government: choose.government.clone(),
                    possible: choose.possible.clone(),
                    selected,
                    needed: choose.needed,
                },
            ))
        },
    )
}

pub fn complete_objectives_dialog(rc: &RenderContext) -> StateUpdate {
    if cancel_button_with_tooltip(rc, "Complete no objectives") {
        return StateUpdate::status_phase(StatusPhaseAction::CompleteObjectives(vec![]));
    }
    StateUpdate::None
}

pub fn determine_first_player_dialog(rc: &RenderContext) -> StateUpdate {
    choose_player_dialog(rc, &rc.game.human_players(), |p| {
        Action::StatusPhase(StatusPhaseAction::DetermineFirstPlayer(p))
    })
}
