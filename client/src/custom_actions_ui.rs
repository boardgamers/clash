use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{payment_dialog, Payment};
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::custom_phase_actions::CustomPhaseAction;
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
