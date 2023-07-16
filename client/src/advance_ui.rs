use std::collections::HashMap;
use server::game::{Action, Game};
use macroquad::ui::root_ui;
use macroquad::hash;
use macroquad::math::{bool, vec2, Vec2};
use server::content::advances::get_technologies;
use macroquad::ui::widgets::Group;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;
use crate::{ActiveDialog, State};
use crate::payment::{new_resource_map, Payment, ResourcePayment, ResourceType};

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
            payment: AdvancePayment::new_advance_resource_payment(
                game.players[player_index]
                    .resources()
                    .get_advance_payment_options(cost),
            ),
            cost,
        }
    }

    pub fn new_advance_resource_payment(a: AdvancePaymentOptions) -> Payment {
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

pub fn buy_advance_menu(game: &mut Game, rp: &mut AdvancePayment) -> bool {
    let mut result = false;
    root_ui().window(hash!(), vec2(20., 510.), vec2(400., 200.), |ui| {
        for (i, p) in rp.payment.resources.iter_mut().enumerate() {
            if p.max > 0 {
                Group::new(hash!("res", i), Vec2::new(70., 200.)).ui(ui, |ui| {
                    let s = format!("{} {}", &p.resource.to_string(), p.current);
                    ui.label(Vec2::new(0., 0.), &s);
                    if p.current > p.min && ui.button(Vec2::new(0., 20.), "-") {
                        p.current -= 1;
                    }
                    if p.current < p.max && ui.button(Vec2::new(20., 20.), "+") {
                        p.current += 1;
                    };
                });
            }
        }

        let label = if rp.valid() { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) && rp.valid() {
            game.execute_action(
                Action::PlayingAction(PlayingAction::Advance {
                    advance: rp.name.clone(),
                    payment: rp.payment.to_resource_pile(),
                }),
                rp.player_index,
            );
            result = true;
        };
    });
    result
}

