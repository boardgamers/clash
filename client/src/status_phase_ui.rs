use crate::advance_ui::{AdvanceState, show_advance_menu};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{OkTooltip, cancel_button_with_tooltip, ok_button};
use crate::render_context::RenderContext;
use server::content::advances::{get_government, get_governments};
use server::content::persistent_events::{ChangeGovernmentRequest, EventResponse};
use server::status_phase::{ChangeGovernment, ChangeGovernmentType};

#[derive(Clone)]
pub struct ChooseAdditionalAdvances {
    government: String,
    possible: Vec<String>,
    selected: Vec<String>,
    needed: usize,
    request: ChangeGovernmentRequest,
}

impl ChooseAdditionalAdvances {
    fn new(
        government: String,
        possible: Vec<String>,
        needed: usize,
        r: &ChangeGovernmentRequest,
    ) -> Self {
        Self {
            government,
            possible,
            selected: Vec::new(),
            needed,
            request: r.clone(),
        }
    }
}

pub fn change_government_type_dialog(
    rc: &RenderContext,
    r: &ChangeGovernmentRequest,
) -> StateUpdate {
    let current = rc.shown_player.government().unwrap();
    if r.optional && cancel_button_with_tooltip(rc, &format!("Keep {current}")) {
        return StateUpdate::response(EventResponse::ChangeGovernmentType(
            ChangeGovernmentType::KeepGovernment,
        ));
    }
    show_advance_menu(
        rc,
        &format!("Change government for {}", r.cost),
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
                .advances
                .iter()
                .skip(1) // the government advance itself is always chosen
                .map(|a| a.name.clone())
                .collect::<Vec<_>>();
            let needed = get_government(&rc.shown_player.government().unwrap())
                .advances
                .iter()
                .filter(|a| rc.shown_player.has_advance(&a.name))
                .count()
                - 1;
            StateUpdate::OpenDialog(ActiveDialog::ChooseAdditionalAdvances(
                ChooseAdditionalAdvances::new(g.clone(), additional, needed, r),
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
        return StateUpdate::response(EventResponse::ChangeGovernmentType(
            ChangeGovernmentType::ChangeGovernment(ChangeGovernment::new(
                choose.government.clone(),
                choose.selected.clone(),
            )),
        ));
    }

    if cancel_button_with_tooltip(rc, "Back to choose government type") {
        return StateUpdate::OpenDialog(ActiveDialog::ChangeGovernmentType(choose.request.clone()));
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
                    request: choose.request.clone(),
                },
            ))
        },
    )
}
