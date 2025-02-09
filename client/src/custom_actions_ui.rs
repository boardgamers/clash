use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::custom_phase_actions::{
    CustomPhaseAdvanceRewardRequest, CustomPhaseEventAction,
};

pub fn payment_reward_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::CustomPhaseResourceRewardRequest(p.clone()),
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CustomPhaseEventAction::ResourceReward(p.clone()),
            ))
        },
    )
}

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
