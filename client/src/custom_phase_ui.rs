use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::hex_ui::Point;
use crate::layout_ui::icon_pos;
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::render_context::RenderContext;
use crate::unit_ui;
use crate::unit_ui::{draw_unit_type, UnitHighlightType};
use macroquad::math::vec2;
use server::action::Action;
use server::content::custom_phase_actions::{
    CustomPhaseAdvanceRewardRequest, CustomPhaseEventAction, CustomPhaseUnitRequest,
};

pub fn custom_phase_payment_dialog(rc: &RenderContext, payments: &[Payment]) -> StateUpdate {
    multi_payment_dialog(
        rc,
        payments,
        |p| ActiveDialog::CustomPhasePaymentRequest(p.clone()),
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
        |p| ActiveDialog::CustomPhaseResourceRewardRequest(p.clone()),
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::ResourceReward(p),
            ))
        },
    )
}

pub fn advance_reward_dialog(
    rc: &RenderContext,
    r: &CustomPhaseAdvanceRewardRequest,
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

pub fn unit_request_dialog(rc: &RenderContext, r: &CustomPhaseUnitRequest) -> StateUpdate {
    let c = &r.choices;
    for (i, u) in c.iter().rev().enumerate() {
        let x = (c.len() - i) as i8 - 3;
        let p = icon_pos(x, -2) + vec2(0., 10.);

        if draw_unit_type(
            rc,
            &UnitHighlightType::None,
            Point::from_vec2(p),
            *u,
            rc.shown_player.index,
            unit_ui::name(u),
            20.,
        ) {
            return StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::SelectUnit(*u),
            ));
        }
    }

    StateUpdate::None
}
