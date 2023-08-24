use std::cmp::min;
use std::collections::HashMap;

use macroquad::hash;
use macroquad::math::{bool, vec2};
use macroquad::ui::root_ui;
use server::action::Action;
use server::content::advances::get_all;
use server::game::Game;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;

use crate::payment_ui::{HasPayment, Payment, payment_dialog, ResourcePayment};
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::ui_state::{can_play_action, StateUpdate, StateUpdates};
use crate::ActiveDialog;
use crate::select_ui::HasSelectableObject;

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
            .map(|e| ResourcePayment::new(e.0, e.1, 0, min(cost, e.1 + left.get(&e.0).unwrap_or(&(0u32)))),
            )
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

pub fn show_advance_menu(game: &Game, player_index: usize) -> StateUpdate {
    let mut updates = StateUpdates::new();

    root_ui().window(hash!(), vec2(20., 900.), vec2(400., 200.), |ui| {
        for a in get_all() {
            let name = a.name;
            let p = game.get_player(player_index);
            if can_play_action(game) && p.can_advance(&name) {
                if ui.button(None, name.clone()) {
                    return updates.add(StateUpdate::SetDialog(ActiveDialog::AdvancePayment(
                        AdvancePayment::new(game, player_index, &name),
                    )));
                }
            } else if p.advances.contains(&name) {
                ui.label(None, &name);
            }
        }
    });

    updates.result()
}

pub fn pay_advance_dialog(ap: &AdvancePayment) -> StateUpdate {
    payment_dialog(
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
