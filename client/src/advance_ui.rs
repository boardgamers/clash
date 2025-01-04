use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::layout_ui::{left_mouse_button_pressed_in_rect, top_center_text};
use crate::payment_ui::{payment_dialog, HasPayment, Payment, ResourcePayment};
use crate::player_ui::player_color;
use crate::resource_ui::{new_resource_map, ResourceType};
use crate::select_ui::HasCountSelectableObject;
use crate::tooltip::show_tooltip_for_rect;
use itertools::Itertools;
use macroquad::math::{bool, vec2, Vec2};
use macroquad::prelude::{
    draw_rectangle, draw_rectangle_lines, Rect, BLACK, BLUE, GRAY, WHITE, YELLOW,
};
use server::action::Action;
use server::advance::{Advance, Bonus};
use server::content::advances;
use server::game::Game;
use server::game::GameState;
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::resource_pile::AdvancePaymentOptions;
use server::status_phase::{StatusPhaseAction, StatusPhaseState};
use std::cmp::min;
use std::collections::HashMap;

#[derive(Clone)]
pub struct AdvancePayment {
    pub name: String,
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

pub fn show_advance_menu(game: &Game, player: &ShownPlayer, state: &State) -> StateUpdate {
    show_generic_advance_menu("Advances", game, player, state, |a| {
        StateUpdate::SetDialog(ActiveDialog::AdvancePayment(AdvancePayment::new(
            game,
            player.index,
            a.name.as_str(),
        )))
    })
}

pub fn show_free_advance_menu(game: &Game, player: &ShownPlayer, state: &State) -> StateUpdate {
    show_generic_advance_menu("Select a free advance", game, player, state, |a| {
        if can_advance(game, player, a) {
            return StateUpdate::execute_with_confirm(
                description(player.get(game), a),
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(a.name.clone())),
            );
        }
        advance_info(game, player, a)
    })
}

fn advance_info(game: &Game, player: &ShownPlayer, a: &Advance) -> StateUpdate {
    StateUpdate::execute_with_cancel(description(player.get(game), a))
}

pub fn show_generic_advance_menu(
    title: &str,
    game: &Game,
    player: &ShownPlayer,
    state: &State,
    new_update: impl Fn(&Advance) -> StateUpdate,
) -> StateUpdate {
    top_center_text(state, title, vec2(0., 10.));
    let p = player.get(game);

    for advances in groups() {
        let pos = group_pos(&advances[0]);
        for (i, a) in advances.into_iter().enumerate() {
            let pos = pos * vec2(140., 210.) + vec2(20., i as f32 * 35. + 50.);
            let name = &a.name;
            let can_advance = can_advance(game, player, &a);

            let fill = if can_advance {
                WHITE
            } else if p.has_advance(name) {
                player_color(player.index)
            } else {
                GRAY
            };
            let rect = Rect::new(pos.x, pos.y, 135., 30.);
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
            state.draw_text(name, pos.x + 25., pos.y + 22.);

            let border_color = if let Some(b) = &a.bonus {
                match b {
                    Bonus::MoodToken => YELLOW,
                    Bonus::CultureToken => BLUE,
                }
            } else {
                BLACK
            };
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 4., border_color);
            show_tooltip_for_rect(state, description(p, &a), rect);

            if can_advance && left_mouse_button_pressed_in_rect(rect, state) {
                return new_update(&a);
            }
        }
    }
    StateUpdate::None
}

fn can_advance(game: &Game, player: &ShownPlayer, a: &Advance) -> bool {
    let name = &a.name;
    let p = player.get(game);
    if player.can_play_action {
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
    }
}

fn groups() -> Vec<Vec<Advance>> {
    let mut current_group = None;
    advances::get_all()
        .into_iter()
        .chunk_by(|a| {
            if a.required.is_none() {
                current_group = Some(a.name.clone());
                a.name.clone()
            } else {
                current_group.as_ref().unwrap().clone()
            }
        })
        .into_iter()
        .map(|(_k, a)| a.collect::<Vec<_>>())
        .collect::<Vec<_>>()
}

fn group_pos(advance: &Advance) -> Vec2 {
    match advance.name.as_str() {
        "Farming" => vec2(0., 0.),
        "Mining" => vec2(1., 0.),
        "Fishing" => vec2(2., 0.),
        "Philosophy" => vec2(3., 0.),
        "Tactics" => vec2(4., 0.),
        "Math" => vec2(2., 1.),
        "Voting" => vec2(3., 1.),
        "Dogma" => vec2(5., 1.),
        _ => panic!("Unknown advance: {}", advance.name),
    }
}

fn description(p: &Player, a: &Advance) -> Vec<String> {
    let name = &a.name;
    let desc = &a.description;

    let mut parts: Vec<String> = vec![];
    parts.push(name.clone());
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

    parts
}

pub fn pay_advance_dialog(
    ap: &AdvancePayment,
    player: &ShownPlayer,
    game: &Game,
    state: &State,
) -> StateUpdate {
    let a = advances::get_advance_by_name(ap.name.as_str()).unwrap();

    if can_advance(game, player, &a) {
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
            state,
        )
    } else {
        advance_info(game, player, &a)
    }
}

fn add(ap: &AdvancePayment, r: ResourceType, i: i32) -> StateUpdate {
    let mut new = ap.clone();
    let p = new.payment.get_mut(r);

    let c = p.counter_mut();
    c.current = (c.current as i32 + i) as u32;
    StateUpdate::SetDialog(ActiveDialog::AdvancePayment(new))
}
