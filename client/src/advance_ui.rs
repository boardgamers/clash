use std::collections::HashMap;

use macroquad::hash;
use macroquad::math::{bool, vec2};
use macroquad::ui::root_ui;
use server::content::advances::get_technologies;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;

use crate::{ActiveDialog, State};
use crate::payment::{HasPayment, new_resource_map, Payment, payment_dialog, ResourcePayment, ResourceType};

pub struct AdvancePayment {
    pub player_index: usize,
    pub name: String,
    pub payment: Payment,
    pub cost: u32,
}

impl AdvancePayment {
    fn new(game: &mut Game, player_index: usize, name: &str) -> AdvancePayment {
        let cost = game.players[player_index].advance_cost(name);
        AdvancePayment {
            player_index,
            name: name.to_string(),
            payment: AdvancePayment::new_payment(
                game.players[player_index]
                    .resources()
                    .get_advance_payment_options(cost),
            ),
            cost,
        }
    }

    pub fn new_payment(a: AdvancePaymentOptions) -> Payment {
        let left = HashMap::from([
            (ResourceType::Food, a.food_left),
            (ResourceType::Gold, a.gold_left),
        ]);

        let mut resources: Vec<ResourcePayment> = new_resource_map(a.default)
            .into_iter()
            .map(|e| ResourcePayment {
                resource: e.0.clone(),
                current: e.1,
                min: 0,
                max: e.1 + left.get(&e.0).unwrap_or(&(0u32)),
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

pub fn show_advance_menu(game: &mut Game, player_index: usize, state: &mut State) {
    root_ui().window(hash!(), vec2(20., 300.), vec2(400., 200.), |ui| {
        for a in get_technologies().into_iter() {
            let name = a.name;
            if game.players[player_index].can_advance(&name) {
                if ui.button(None, name.clone()) {
                    state.active_dialog = ActiveDialog::AdvancePayment(AdvancePayment::new(game, player_index, &name));
                }
            } else if game.players[player_index].advances.contains(&name) {
                ui.label(None, &name);
            }
        }
    });
}

pub fn pay_advance_dialog(game: &mut Game, ap: &mut AdvancePayment) -> bool {
    payment_dialog(ap,
                   |ap| ap.valid(),
                   |ap| game.execute_action(
                       Action::PlayingAction(PlayingAction::Advance {
                           advance: ap.name.clone(),
                           payment: ap.payment.to_resource_pile(),
                       }),
                       ap.player_index,
                   ),
                   |ap, r| {
                       add(ap, r, 1);
                   },
                   |ap, r| {
                       add(ap, r, -1);
                   }
    )
}

fn add(ap: &mut AdvancePayment, r: ResourceType, i: i32) {
    let x = ap.payment.resources.iter_mut().find(|p| p.resource == r).unwrap();
    x.current = (x.current as i32 + i) as u32
}

