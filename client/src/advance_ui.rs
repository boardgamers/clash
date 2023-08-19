use std::cmp::min;
use std::collections::HashMap;

use macroquad::hash;
use macroquad::math::{bool, vec2};
use macroquad::ui::root_ui;
use server::action::Action;
use server::content::advances::get_technologies;
use server::game::Game;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;

use crate::payment_ui::{payment_dialog, HasPayment, Payment, ResourcePayment};
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::ui_state::{can_play_action, StateUpdate, StateUpdates};
use crate::{ActiveDialog, State};

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
                p.resources().get_advance_payment_options(cost),
                cost,
            ),
            cost,
        }
    }

    pub fn new_payment(a: AdvancePaymentOptions, cost: u32) -> Payment {
        let left = HashMap::from([
            (ResourceType::Food, a.food_left),
            (ResourceType::Gold, a.gold_left),
        ]);

        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
            .into_iter()
            .map(|e| ResourcePayment {
                resource: e.0.clone(),
                current: e.1,
                min: 0,
                max: min(cost, e.1 + left.get(&e.0).unwrap_or(&(0u32))),
            })
            .collect();
        resources.sort_by_key(|r| r.resource.clone());

        Payment { resources }
    }

    pub fn valid(&self) -> bool {
        self.payment
            .resources
            .iter()
            .map(|r| r.current)
            .sum::<u32>()
            == self.cost
    }
}

impl HasPayment for AdvancePayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}

pub fn show_advance_menu<'a>(game: &'a Game, player_index: usize) -> StateUpdate<'a> {
    let mut updates = StateUpdates::new();

    root_ui().window(hash!(), vec2(20., 900.), vec2(400., 200.), |ui| {
        for a in get_technologies().into_iter() {
            let name = a.name;
            let p = game.get_player(player_index);
            if can_play_action(game) && p.can_advance(&name) {
                if ui.button(None, name.clone()) {
                    return updates.add(StateUpdate::NewDialog(ActiveDialog::AdvancePayment(
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

pub fn pay_advance_dialog<'a>(ap: &AdvancePayment) -> StateUpdate<'a> {
    payment_dialog(
        ap,
        |ap| ap.valid(),
        |ap| {
            StateUpdate::Execute(Action::Playing(PlayingAction::Advance {
                advance: ap.name.to_string(),
                payment: ap.payment.to_resource_pile(),
            }))
        },
        |ap, r| ap.payment.get(r).max > 0,
        |ap, r| {
            add(r, 1)
        },
        |ap, r| {
            add(r, -1)
        },
    )
}

fn add<'a>(r: ResourceType, i: i32) -> StateUpdate<'a> {
    StateUpdate::UpdateActiveDialog(Box::new(|ad| {
        if let ActiveDialog::AdvancePayment(ap) = ad {
            let p = ap.payment.get_mut(r);
            p.current = (p.current as i32 + i) as u32
        }
    }))
}
