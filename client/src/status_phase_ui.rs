use crate::advance_ui::{AdvanceState, show_advance_menu};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{OkTooltip, cancel_button_with_tooltip, ok_button};
use crate::render_context::RenderContext;
use server::advance::Advance;
use server::content::persistent_events::{ChangeGovernmentRequest, EventResponse};
use server::status_phase::{ChangeGovernment, ChangeGovernmentType};

#[derive(Clone)]
pub struct ChooseAdditionalAdvances {
    government: String,
    possible: Vec<Advance>,
    selected: Vec<Advance>,
    needed: usize,
}

impl ChooseAdditionalAdvances {
    fn new(
        government: String,
        possible: Vec<Advance>,
        needed: usize,
    ) -> Self {
        Self {
            government,
            possible,
            selected: Vec::new(),
            needed,
        }
    }
}

pub fn change_government_type_dialog(rc: &RenderContext) -> StateUpdate {
    show_advance_menu(
        rc,
        "Change government",
        |a, p| {
            if rc
                .game
                .cache
                .get_governments()
                .iter()
                .find(|g| g.advances[0].name == a.name)
                .is_some_and(|_| p.can_advance_ignore_contradicting(a.advance, rc.game))
            {
                AdvanceState::Available
            } else if rc.shown_player.has_advance(a.advance) && a.government.is_some() {
                AdvanceState::Owned
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            let g = a.government.as_ref().expect("should have government");
            let additional = rc
                .game
                .cache
                .get_government(g)
                .advances
                .iter()
                .skip(1) // the government advance itself is always chosen
                .map(|a| a.advance)
                .collect::<Vec<_>>();
            let needed = rc
                .game
                .cache
                .get_government(&rc.shown_player.government(rc.game).unwrap())
                .advances
                .iter()
                .filter(|a| rc.shown_player.has_advance(a.advance))
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
            ChangeGovernment::new(
                choose.government.clone(),
                choose.selected.clone(),
            ),
        ));
    }

    if cancel_button_with_tooltip(rc, "Back to choose government type") {
        return StateUpdate::OpenDialog(ActiveDialog::ChangeGovernmentType(choose.request.clone()));
    }
    show_advance_menu(
        rc,
        &format!("Choose additional advances for {}", choose.government),
        |a, _| {
            if choose.selected.contains(&a.advance) {
                AdvanceState::Removable
            } else if choose.possible.contains(&a.advance) && choose.selected.len() < choose.needed
            {
                AdvanceState::Available
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            let mut selected = choose.selected.clone();
            if selected.contains(&a.advance) {
                selected.retain(|n| n != &a.advance);
            } else {
                selected.push(a.advance);
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
