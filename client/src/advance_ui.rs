use crate::city_ui::add_building_description;
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::layout_ui::{left_mouse_button_pressed_in_rect, top_centered_text};
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use crate::tooltip::{add_tooltip_description, show_tooltip_for_rect};
use crate::unit_ui::add_unit_description;
use itertools::Itertools;
use macroquad::color::Color;
use macroquad::math::vec2;
use macroquad::prelude::{
    BLACK, BLUE, GRAY, GREEN, Rect, WHITE, YELLOW, draw_rectangle, draw_rectangle_lines,
};
use server::action::Action;
use server::advance::{Advance, AdvanceAction, AdvanceInfo, Bonus, find_special_advance};
use server::game::GameState;
use server::player::{CostTrigger, Player};
use server::playing_actions::PlayingAction;
use server::unit::UnitType;
use std::ops::Rem;

const COLUMNS: usize = 6;

pub enum AdvanceState {
    Owned,
    Removable,
    Available,
    Unavailable,
}

fn new_advance_payment(rc: &RenderContext, a: &AdvanceInfo) -> Payment<Advance> {
    rc.new_payment(
        &rc.shown_player
            .advance_cost(a.advance, rc.game, CostTrigger::WithModifiers)
            .cost,
        a.advance,
        &a.name,
        false,
    )
}

pub fn show_paid_advance_menu(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    show_advance_menu(
        rc,
        "Advances",
        |a, p| {
            if p.has_advance(a.advance) {
                AdvanceState::Owned
            } else if game.state == GameState::Playing
                && game.actions_left > 0
                && p.can_advance(a.advance, game)
            {
                AdvanceState::Available
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| StateUpdate::OpenDialog(ActiveDialog::AdvancePayment(new_advance_payment(rc, a))),
    )
}

pub fn show_advance_menu(
    rc: &RenderContext,
    title: &str,
    advance_state: impl Fn(&AdvanceInfo, &Player) -> AdvanceState,
    new_update: impl Fn(&AdvanceInfo) -> StateUpdate,
) -> StateUpdate {
    top_centered_text(rc, title, vec2(0., 10.));
    let p = rc.shown_player;
    let state = rc.state;

    for pass in 0..2 {
        for (i, group) in rc.game.cache.get_advance_groups().iter().enumerate() {
            let pos =
                vec2(i.rem(COLUMNS) as f32 * 140., (i / COLUMNS) as f32 * 180.) + vec2(20., 70.);
            if pass == 0 {
                state.draw_text(
                    &group.name,
                    pos.x + (140. - state.measure_text(&group.name).width) / 2.,
                    pos.y - 15.,
                );
            }

            for (i, a) in group.advances.iter().enumerate() {
                let pos = pos + vec2(0., i as f32 * 35.);
                let name = &a.name;
                let advance_state = advance_state(a, p);

                let rect = Rect::new(pos.x, pos.y, 135., 30.);
                if pass == 0 {
                    draw_rectangle(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        fill_color(rc, p, &advance_state),
                    );
                    state.draw_text(name, pos.x + 10., pos.y + 22.);

                    if find_special_advance(a.advance, rc.game, rc.shown_player.index).is_some() {
                        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 12., GREEN);
                    }

                    draw_rectangle_lines(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        match &state.active_dialog {
                            ActiveDialog::AdvancePayment(p) => {
                                if p.name == *name {
                                    8.
                                } else {
                                    4.
                                }
                            }
                            _ => 4.,
                        },
                        border_color(a),
                    );
                } else {
                    // tooltip should be shown on top of everything
                    show_tooltip_for_rect(rc, &description(rc, a), rect, 50.);

                    if rc.can_control_shown_player()
                        && matches!(
                            advance_state,
                            AdvanceState::Available | AdvanceState::Removable
                        )
                        && left_mouse_button_pressed_in_rect(rect, rc)
                    {
                        return new_update(a);
                    }
                }
            }
        }
    }
    StateUpdate::None
}

fn fill_color(rc: &RenderContext, p: &Player, advance_state: &AdvanceState) -> Color {
    match advance_state {
        AdvanceState::Owned | AdvanceState::Removable => rc.player_color(p.index),
        AdvanceState::Available => WHITE,
        AdvanceState::Unavailable => GRAY,
    }
}

fn border_color(a: &AdvanceInfo) -> Color {
    if let Some(b) = &a.bonus {
        match b {
            Bonus::MoodToken => YELLOW,
            Bonus::CultureToken => BLUE,
        }
    } else {
        BLACK
    }
}

fn description(rc: &RenderContext, a: &AdvanceInfo) -> Vec<String> {
    let mut parts: Vec<String> = vec![];
    parts.push(a.name.clone());
    add_tooltip_description(&mut parts, &a.description);
    parts.push(format!(
        "Cost: {}",
        rc.shown_player
            .advance_cost(a.advance, rc.game, CostTrigger::WithModifiers)
            .cost
    ));
    if let Some(r) = &a.required {
        parts.push(format!("Required: {}", r.name(rc.game)));
    }
    if !a.contradicting.is_empty() {
        parts.push(format!(
            "Contradicts: {}",
            a.contradicting.iter().map(|a| a.name(rc.game)).join(", ")
        ));
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
    if let Some(b) = &a.unlocked_building {
        parts.push(format!("Unlocks building: {}", b.name()));
        add_building_description(rc, &mut parts, *b);
    }
    if a.advance == Advance::Bartering {
        parts.push("Can build in a city with a Market: cavalry".to_string());
        add_unit_description(rc, &mut parts, UnitType::Cavalry);
        parts.push("Can build in a city with a Market: elephant".to_string());
        add_unit_description(rc, &mut parts, UnitType::Elephant);
    }

    if let Some(a) = find_special_advance(a.advance, rc.game, rc.shown_player.index) {
        let s = a.info(rc.game);
        parts.push(format!("Special advance: {}", s.name));
        add_tooltip_description(&mut parts, &s.description);
    }

    parts
}

pub fn pay_advance_dialog(ap: &Payment<Advance>, rc: &RenderContext) -> StateUpdate {
    let update = show_paid_advance_menu(rc);
    if !matches!(update, StateUpdate::None) {
        // select a different advance
        return update;
    }
    payment_dialog(rc, ap, true, ActiveDialog::AdvancePayment, |payment| {
        StateUpdate::execute_with_warning(
            Action::Playing(PlayingAction::Advance(AdvanceAction::new(
                ap.value, payment,
            ))),
            if rc.shown_player.incident_tokens == 1 {
                vec!["A game event will be triggered".to_string()]
            } else {
                vec![]
            },
        )
    })
}
