use crate::client_state::{ActiveDialog, StateUpdate};
use crate::layout_ui::{left_mouse_button_pressed_in_rect, top_centered_text};
use crate::log_ui::break_text;
use crate::payment_ui::{Payment, payment_dialog};
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_rect;
use macroquad::color::Color;
use macroquad::math::vec2;
use macroquad::prelude::{
    BLACK, BLUE, GRAY, Rect, WHITE, YELLOW, draw_rectangle, draw_rectangle_lines,
};
use server::action::Action;
use server::advance::{Advance, Bonus};
use server::content::advances;
use server::game::GameState;
use server::player::Player;
use server::playing_actions::PlayingAction;
use std::ops::Rem;

const COLUMNS: usize = 6;

pub enum AdvanceState {
    Owned,
    Removable,
    Available,
    Unavailable,
}

fn new_advance_payment(rc: &RenderContext, a: &Advance) -> Payment {
    rc.new_payment(&rc.shown_player.advance_cost(a, None).cost, &a.name, false)
}

pub fn show_paid_advance_menu(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    show_advance_menu(
        rc,
        "Advances",
        |a, p| {
            if p.has_advance(&a.name) {
                AdvanceState::Owned
            } else if game.state == GameState::Playing && game.actions_left > 0 && p.can_advance(a)
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
    advance_state: impl Fn(&Advance, &Player) -> AdvanceState,
    new_update: impl Fn(&Advance) -> StateUpdate,
) -> StateUpdate {
    top_centered_text(rc, title, vec2(0., 10.));
    let p = rc.shown_player;
    let state = rc.state;

    for pass in 0..2 {
        for (i, group) in advances::get_groups().iter().enumerate() {
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

                    let thickness = match &state.active_dialog {
                        ActiveDialog::AdvancePayment(p) => {
                            if p.name == *name {
                                8.
                            } else {
                                4.
                            }
                        }
                        _ => 4.,
                    };
                    draw_rectangle_lines(
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        thickness,
                        border_color(a),
                    );
                } else {
                    // tooltip should be shown on top of everything
                    show_tooltip_for_rect(rc, &description(p, a), rect, 50.);

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

fn border_color(a: &Advance) -> Color {
    if let Some(b) = &a.bonus {
        match b {
            Bonus::MoodToken => YELLOW,
            Bonus::CultureToken => BLUE,
        }
    } else {
        BLACK
    }
}

fn description(p: &Player, a: &Advance) -> Vec<String> {
    let desc = &a.description;

    let mut parts: Vec<String> = vec![];
    parts.push(a.name.clone());
    break_text(desc, 70, &mut parts);
    parts.push(format!("Cost: {}", p.advance_cost(a, None).cost));
    if let Some(r) = &a.required {
        parts.push(format!("Required: {r}"));
    }
    if !a.contradicting.is_empty() {
        parts.push(format!("Contradicts: {}", a.contradicting.join(", ")));
    }
    if let Some(b) = &a.bonus {
        parts.push(format!("Bonus: {}", match b {
            Bonus::MoodToken => "Mood Token",
            Bonus::CultureToken => "Culture Token",
        }));
    }
    if let Some(g) = &a.government {
        parts.push(format!("Government: {g}"));
    }
    if let Some(u) = &a.unlocked_building {
        parts.push(format!("Unlocks: {}", u.name()));
    }

    parts
}

pub fn pay_advance_dialog(ap: &Payment, rc: &RenderContext) -> StateUpdate {
    let update = show_paid_advance_menu(rc);
    if !matches!(update, StateUpdate::None) {
        // select a different advance
        return update;
    };
    payment_dialog(rc, ap, true, ActiveDialog::AdvancePayment, |payment| {
        StateUpdate::Execute(Action::Playing(PlayingAction::Advance {
            advance: ap.name.to_string(),
            payment,
        }))
    })
}
