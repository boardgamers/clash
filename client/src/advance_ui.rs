use std::cmp::min;
use std::collections::HashMap;

use macroquad::math::bool;

use server::action::Action;
use server::advance::{Advance, Bonus};
use server::content::advances::get_all;
use server::game::Game;
use server::game::GameState;
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};

use crate::client_state::{ActiveDialog, ShownPlayer, StateUpdate};
use crate::dialog_ui::full_dialog;
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
    show_generic_advance_menu("Advances", game, player, |name| {
        StateUpdate::SetDialog(ActiveDialog::AdvancePayment(AdvancePayment::new(
            game,
            player.index,
            name,
        )))
    })
}

pub fn show_free_advance_menu(game: &Game, player: &ShownPlayer) -> StateUpdate {
    show_generic_advance_menu("Select a free advance", game, player, |name| {
        StateUpdate::status_phase(StatusPhaseAction::FreeAdvance(name.to_string()))
    })
}

pub fn show_generic_advance_menu(
    title: &str,
    game: &Game,
    player: &ShownPlayer,
    new_update: impl Fn(&str) -> StateUpdate,
) -> StateUpdate {
    full_dialog(title, |ui| {
        let p = player.get(game);
        for a in get_all() {
            let name = &a.name;
            let can_advance = if player.can_play_action {
                p.can_advance(name)
            } else if player.can_control
                && matches!(
                    game.state,
                    GameState::StatusPhase(StatusPhaseState::FreeAdvance)
                )
            {
                p.can_advance_free(name)
            } else {
                false
            };

            let desc = description(p, &a);
            if p.has_advance(name) {
                ui.label(None, &desc);
            } else if can_advance {
                if ui.button(None, desc) {
                    return new_update(name);
                }
            } else {
                ui.label(None, &desc);
            };
        }
        StateUpdate::None
    })
}

fn description(p: &Player, a: &Advance) -> String {
    let name = &a.name;
    let desc = &a.description;

    let mut parts = vec![];
    parts.push(if p.has_advance(name) {
        format!("+ {name}")
    } else {
        format!("  {name}")
    });
    parts.push(desc.clone());
    parts.push(format!("Cost: {}", p.advance_cost(name)));
    if let Some(r) = &a.required {
        parts.push(format!("Required: {r}"));
    }
    if let Some(c) = &a.contradicting {
        parts.push(format!("Contradicts: {c}"));
    }
    if let Some(b) = &a.bonus {
        parts.push(format!(
            "Bonus: {}",
            match b {
                Bonus::MoodToken => "Mood Token",
                Bonus::CultureToken => "Culture Token",
            }
        ));
    }
    if let Some(g) = &a.government {
        parts.push(format!("Government: {g}"));
    }
    if let Some(u) = &a.unlocked_building {
        parts.push(format!("Unlocks: {u}"));
    }

    parts.join(", ")
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
