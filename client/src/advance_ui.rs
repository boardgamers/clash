use std::cmp::min;
use std::collections::HashMap;

use macroquad::math::bool;

use server::action::Action;
use server::content::advances::get_all;
use server::game::Game;
use server::game::GameState;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};

use crate::client_state::{ActiveDialog, ShownPlayer, StateUpdate};
use crate::dialog_ui::{ dialog};
use crate::payment_ui::{payment_dialog, HasPayment, Payment, ResourcePayment};
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::select_ui::HasCountSelectableObject;

#[derive(Clone)]
pub struct AdvancePayment {
    name: String,
    payment: Payment,
    cost: u32,
}

impl AdvancePayment {
    fn new(game: &Game, player_index: usize, name: &str) -> AdvancePayment {
        let p = game.get_player(player_index);
        let cost = p.advance_cost(name);
        AdvancePayment {
            name: name.to_string(),
            payment: AdvancePayment::new_payment(
                &p.resources.get_advance_payment_options(cost),
                cost,
            ),
            cost,
        }
    }

    pub fn new_payment(a: &AdvancePaymentOptions, cost: u32) -> Payment {
        let left = HashMap::from([
            (ResourceType::Food, a.food_left),
            (ResourceType::Gold, a.gold_left),
        ]);

        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
            .into_iter()
            .map(|e| {
                ResourcePayment::new(
                    e.0,
                    e.1,
                    0,
                    min(cost, e.1 + left.get(&e.0).unwrap_or(&(0u32))),
                )
            })
            .collect();
        resources.sort_by_key(|r| r.resource);

        Payment { resources }
    }

    pub fn valid(&self) -> bool {
        self.payment
            .resources
            .iter()
            .map(|r| r.selectable.current)
            .sum::<u32>()
            == self.cost
    }
}

impl HasPayment for AdvancePayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}

pub fn show_advance_menu(game: &Game, player: &ShownPlayer) -> StateUpdate {
    show_generic_advance_menu("Advances", game, player, true, |name| {
        StateUpdate::SetDialog(ActiveDialog::AdvancePayment(AdvancePayment::new(
            game,
            player.index,
            &name,
        )))
    })
}

pub fn show_free_advance_menu(game: &Game, player: &ShownPlayer) -> StateUpdate {
    show_generic_advance_menu("Select a free advance", game, player, false, |name| {
        StateUpdate::status_phase(StatusPhaseAction::FreeAdvance(name))
    })
}

pub fn show_generic_advance_menu(
    title: &str,
    game: &Game,
    player: &ShownPlayer,
    close_button: bool,
    new_update: impl Fn(String) -> StateUpdate,
) -> StateUpdate {
    dialog(title, close_button, |ui| {
        let p = player.get(game);
        for a in get_all() {
            let name = a.name;
            if player.can_control {
                if p.has_advance(&name) {
                    ui.label(None, &name);
                } else {
                    let can = if matches!(
                        game.state,
                        GameState::StatusPhase(StatusPhaseState::FreeAdvance)
                    ) {
                        p.can_advance_free(&name)
                    } else {
                        player.can_control && p.can_advance(&name)
                    };
                    if can && ui.button(None, name.clone()) {
                        return new_update(name);
                    }
                }
            } else  if p.has_advance(&name) { 
                ui.label(None, &name);
            }
        }
        StateUpdate::None
    })
}

pub fn pay_advance_dialog(ap: &AdvancePayment, player: &ShownPlayer) -> StateUpdate {
    payment_dialog(
        player,
        &format!("Pay for advance {}", ap.name),
        ap,
        AdvancePayment::valid,
        |ap| {
            StateUpdate::Execute(Action::Playing(PlayingAction::Advance {
                advance: ap.name.to_string(),
                payment: ap.payment.to_resource_pile(),
            }))
        },
        |ap, r| ap.payment.get(r).selectable.max > 0,
        |ap, r| add(ap, r, 1),
        |ap, r| add(ap, r, -1),
    )
}

fn add(ap: &AdvancePayment, r: ResourceType, i: i32) -> StateUpdate {
    let mut new = ap.clone();
    let p = new.payment.get_mut(r);

    let c = p.counter_mut();
    c.current = (c.current as i32 + i) as u32;
    StateUpdate::SetDialog(ActiveDialog::AdvancePayment(new))
}
