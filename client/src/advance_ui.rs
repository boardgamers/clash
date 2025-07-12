use crate::city_ui::add_building_description;
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::layout_ui::{button_pressed, top_centered_text};
use crate::log_ui::{ MultilineText};
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use crate::unit_ui::add_unit_description;
use itertools::Itertools;
use macroquad::color::Color;
use macroquad::math::vec2;
use macroquad::prelude::{BLACK, BLUE, GRAY, GREEN, Rect, WHITE, YELLOW};
use server::action::Action;
use server::advance::{Advance, AdvanceAction, AdvanceInfo, Bonus, find_special_advance};
use server::game::GameState;
use server::player::{CostTrigger, Player};
use server::playing_actions::PlayingAction;
use server::unit::UnitType;
use std::ops::Rem;

const COLUMNS: usize = 6;

pub(crate) enum AdvanceState {
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

pub(crate) fn show_paid_advance_menu(rc: &RenderContext) -> RenderResult {
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
        |a| StateUpdate::open_dialog(ActiveDialog::AdvancePayment(new_advance_payment(rc, a))),
    )
}

pub(crate) fn show_advance_menu(
    rc: &RenderContext,
    title: &str,
    advance_state: impl Fn(&AdvanceInfo, &Player) -> AdvanceState,
    new_update: impl Fn(&AdvanceInfo) -> RenderResult,
) -> RenderResult {
    top_centered_text(rc, title, vec2(0., 10.));
    let p = rc.shown_player;
    let state = rc.state;

    for (i, group) in rc.game.cache.get_advance_groups().iter().enumerate() {
        let pos = vec2(i.rem(COLUMNS) as f32 * 140., (i / COLUMNS) as f32 * 180.) + vec2(20., 70.);
        rc.draw_text(
            &group.name,
            pos.x + (140. - state.measure_text(&group.name).width) / 2.,
            pos.y - 15.,
        );

        for (i, a) in group.advances.iter().enumerate() {
            let pos = pos + vec2(0., i as f32 * 35.);
            let name = &a.name;
            let advance_state = advance_state(a, p);

            let rect = Rect::new(pos.x, pos.y, 135., 30.);
            rc.draw_rectangle_with_text(rect, fill_color(rc, p, &advance_state), name);

            if find_special_advance(a.advance, rc.game, rc.shown_player.index).is_some() {
                rc.draw_rectangle_lines(rect, 12., GREEN);
            }

            rc.draw_rectangle_lines(
                rect,
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
            // tooltip should be shown on top of everything
            if button_pressed(rect, rc, &description(rc, a), 50.)
                && rc.can_control_shown_player()
                && matches!(
                    advance_state,
                    AdvanceState::Available | AdvanceState::Removable
                )
            {
                return new_update(a);
            }
        }
    }
    NO_UPDATE
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

fn description(rc: &RenderContext, a: &AdvanceInfo) -> MultilineText {
    let mut parts = MultilineText::default();
    parts.add(rc, &a.name);
    parts.add(rc, &a.description);
    parts.add(rc, &format!(
        "Cost: {}",
        rc.shown_player
            .advance_cost(a.advance, rc.game, CostTrigger::WithModifiers)
            .cost
    ));
    if let Some(r) = &a.required {
        parts.add(rc, &format!("Required: {}", r.name(rc.game)));
    }
    if !a.contradicting.is_empty() {
        parts.add(rc, &format!(
            "Contradicts: {}",
            a.contradicting.iter().map(|a| a.name(rc.game)).join(", ")
        ));
    }
    if let Some(b) = &a.bonus {
        parts.add(rc, &format!(
            "Bonus: {}",
            match b {
                Bonus::MoodToken => "Mood Token",
                Bonus::CultureToken => "Culture Token",
            }
        ));
    }
    if let Some(g) = &a.government {
        parts.add(rc, &format!("Government: {g}"));
    }
    if let Some(b) = &a.unlocked_building {
        parts.add(rc, &format!("Unlocks building: {}", b.name()));
        add_building_description(rc, &mut parts, *b);
    }
    if a.advance == Advance::Bartering {
        parts.add(rc, "Can build in a city with a Market: cavalry");
        add_unit_description(rc, &mut parts, UnitType::Cavalry);
        parts.add(rc, "Can build in a city with a Market: elephant");
        add_unit_description(rc, &mut parts, UnitType::Elephant);
    }

    if let Some(a) = find_special_advance(a.advance, rc.game, rc.shown_player.index) {
        let s = a.info(rc.game);
        parts.add(rc, &format!("Special advance: {}", s.name));
        parts.add(rc, &s.description);
    }

    parts
}

pub(crate) fn pay_advance_dialog(ap: &Payment<Advance>, rc: &RenderContext) -> RenderResult {
    show_paid_advance_menu(rc)?; // select a different advance
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
