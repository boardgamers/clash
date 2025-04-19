use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::custom_actions::CustomAction;
use server::playing_actions::PlayingAction;

pub fn taxes(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::Taxes(p.clone()),
        |p| {
            StateUpdate::Execute(Action::Playing(PlayingAction::Custom(CustomAction::Taxes(
                p,
            ))))
        },
    )
}
