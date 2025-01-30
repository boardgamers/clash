use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::custom_phase_actions::{CustomPhaseAction, CustomPhaseEventAction};
use server::content::trade_routes::trade_route_reward;
use server::game::Game;

pub fn trade_route_dialog(game: &Game) -> ActiveDialog {
    let model = trade_route_reward(game).unwrap().0;
    ActiveDialog::TradeRouteSelection(Payment::new_gain(model, "Select trade route reward"))
}

pub fn trade_route_selection_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        |p| ActiveDialog::TradeRouteSelection(p.clone()),
        |p| {
            StateUpdate::Execute(Action::CustomPhase(
                CustomPhaseAction::TradeRouteSelectionAction(p),
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
