use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::hex_ui::Point;
use crate::layout_ui::{bottom_center_anchor, icon_pos};
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::render_context::RenderContext;
use crate::select_ui::ConfirmSelection;
use crate::unit_ui;
use crate::unit_ui::{draw_unit_type, UnitHighlightType, UnitSelection};
use macroquad::math::vec2;
use server::action::Action;
use server::content::custom_phase_actions::{
    AdvanceRewardRequest, CustomPhaseEventAction, UnitTypeRequest,
};
use server::game::Game;
use server::unit::Unit;

pub fn custom_phase_payment_dialog(rc: &RenderContext, payments: &[Payment]) -> StateUpdate {
    multi_payment_dialog(
        rc,
        payments,
        |p| ActiveDialog::PaymentRequest(p.clone()),
        false,
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(
                p.clone(),
            )))
        },
    )
}

pub fn payment_reward_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::ResourceRewardRequest(p.clone()),
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::ResourceReward(p),
            ))
        },
    )
}

pub fn advance_reward_dialog(
    rc: &RenderContext,
    r: &AdvanceRewardRequest,
    name: &str,
) -> StateUpdate {
    let possible = &r.choices;
    show_advance_menu(
        rc,
        &format!("Select advance for {name}"),
        |a, _| {
            if possible.contains(&a.name) {
                AdvanceState::Available
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::AdvanceReward(a.name.clone()),
            ))
        },
    )
}

pub fn unit_request_dialog(rc: &RenderContext, r: &UnitTypeRequest) -> StateUpdate {
    let c = &r.choices;
    let anchor = bottom_center_anchor(rc) + vec2(0., 60.);
    for (i, u) in c.iter().enumerate() {
        let x = (c.len() - i) as i8 - 1;
        let p = icon_pos(x, -2) + anchor;

        if draw_unit_type(
            rc,
            &UnitHighlightType::None,
            Point::from_vec2(p),
            *u,
            r.player_index,
            unit_ui::name(u),
            20.,
        ) {
            return StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::SelectUnitType(*u),
            ));
        }
    }

    StateUpdate::None
}

#[derive(Clone)]
pub struct UnitsSelection {
    pub needed: u8,
    pub selectable: Vec<u32>,
    pub units: Vec<u32>,
    pub description: Option<String>,
}

impl UnitsSelection {
    pub fn new(needed: u8, selectable: Vec<u32>, description: Option<String>) -> Self {
        UnitsSelection {
            needed,
            units: Vec::new(),
            selectable,
            description,
        }
    }
}

impl UnitSelection for UnitsSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selectable.contains(&unit.id)
    }
}

impl ConfirmSelection for UnitsSelection {
    fn cancel_name(&self) -> Option<&str> {
        None
    }

    fn confirm(&self, _game: &Game) -> OkTooltip {
        if self.units.len() as u8 == self.needed {
            OkTooltip::Valid("Select units".to_string())
        } else {
            OkTooltip::Invalid(format!(
                "Need to select {} units",
                self.needed - self.units.len() as u8
            ))
        }
    }
}

pub fn select_units_dialog(rc: &RenderContext, sel: &UnitsSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<UnitsSelection>(rc, sel, |new: UnitsSelection| {
        StateUpdate::Execute(Action::CustomPhaseEvent(
            CustomPhaseEventAction::SelectUnits(new.units.clone()),
        ))
    })
}

pub(crate) fn bool_request_dialog(rc: &RenderContext) -> StateUpdate {
    if ok_button(rc, OkTooltip::Valid("OK".to_string())) {
        return bool_answer(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return bool_answer(false);
    }
    StateUpdate::None
}

fn bool_answer(answer: bool) -> StateUpdate {
    StateUpdate::Execute(Action::CustomPhaseEvent(CustomPhaseEventAction::Bool(
        answer,
    )))
}
