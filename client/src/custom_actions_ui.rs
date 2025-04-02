use crate::client_state::{ActiveDialog, StateUpdate};
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use server::action::Action;
use server::content::custom_actions::CustomAction;
use server::playing_actions::PlayingAction;
use server::position::Position;

pub fn sports(rc: &RenderContext, payment: &Payment, pos: Position) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::Sports((p.clone(), pos)),
        |p| {
            StateUpdate::Execute(Action::Playing(PlayingAction::Custom(
                CustomAction::Sports {
                    city_position: pos,
                    payment: p,
                },
            )))
        },
    )
}

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

pub fn theaters(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::Theaters(p.clone()),
        |p| {
            StateUpdate::Execute(Action::Playing(PlayingAction::Custom(
                CustomAction::Theaters(p),
            )))
        },
    )
}
