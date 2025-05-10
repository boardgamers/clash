use crate::action_buttons::action_buttons;
use crate::city_ui::city_labels;
use crate::client::Features;
use crate::client_state::StateUpdate;
use crate::dialog_ui::{OkTooltip, ok_button};
use crate::layout_ui::{
    ICON_SIZE, bottom_center_texture, bottom_centered_text, bottom_right_texture, icon_pos,
    left_mouse_button_pressed_in_rect, top_center_anchor, top_center_texture,
};
use crate::log_ui::multiline_label;
use crate::map_ui::terrain_name;
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::tooltip::{show_tooltip_for_circle, show_tooltip_for_rect};
use crate::unit_ui;
use itertools::Itertools;
use macroquad::math::vec2;
use macroquad::prelude::*;
use server::action::Action;
use server::combat_stats::CombatStats;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::content::persistent_events::PersistentEventType;
use server::game::{Game, GameState};
use server::movement::{CurrentMove, MovementAction};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource::ResourceType;
use server::status_phase::get_status_phase;

pub fn player_select(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let players = game.human_players(game.starting_player_index);

    let size = 50.;
    let mut y = (players.len() as f32 * -size) / 2.;

    for player_index in players {
        let pl = game.player(player_index);
        let shown = rc.shown_player.index == pl.index;
        let screen = rc.state.screen_size;
        let pos = vec2(screen.x, screen.y / 2.0) + vec2(-size, y);

        let color = rc.player_color(pl.index);

        let w = if shown { size + 10. } else { size };
        let x = pos.x - w + size;
        draw_rectangle(x, pos.y, w, size, color);
        draw_rectangle_lines(x, pos.y, w, size, 2.0, BLACK);
        let text = format!("{}", pl.victory_points(game));

        let state = rc.state;
        state.draw_text(&text, pos.x + 10., pos.y + 27.);

        if game.active_player() == pl.index {
            draw_texture_ex(
                &rc.assets().active_player,
                x - 20.,
                pos.y + 13.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20., 20.)),
                    ..Default::default()
                },
            );
        }

        let rect = Rect::new(x, pos.y, w, size);
        let tooltip = if state.control_player.is_some_and(|p| p == pl.index) {
            format!("{pl} (You)")
        } else {
            pl.get_name()
        };
        show_tooltip_for_rect(rc, &[tooltip], rect, 50.);
        if !shown && left_mouse_button_pressed_in_rect(rect, rc) {
            return StateUpdate::SetShownPlayer(pl.index);
        }

        y += size;
    }

    StateUpdate::None
}

pub fn top_icon_with_label(
    rc: &RenderContext,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) {
    let state = rc.state;
    let dimensions = state.measure_text(label);
    let x = (ICON_SIZE - dimensions.width) / 2.0;
    state.draw_text(
        label,
        state.screen_size.x / 2.0 + p.x + x,
        p.y + ICON_SIZE + 30.,
    );
    top_center_texture(rc, texture, p, tooltip);
}

pub fn bottom_icon_with_label(
    rc: &RenderContext,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) {
    let state = rc.state;
    let dimensions = state.measure_text(label);
    let x = (ICON_SIZE - dimensions.width) / 2.0;
    state.draw_text(
        label,
        rc.state.screen_size.x / 2.0 + p.x + x,
        rc.state.screen_size.y + p.y + 35.,
    );
    bottom_center_texture(rc, texture, p, tooltip);
}

pub fn show_top_center(rc: &RenderContext) {
    let player = rc.shown_player;

    let pos = icon_pos(3, 0);
    top_icon_with_label(
        rc,
        &format!("{}", &player.victory_points(rc.game)),
        &rc.assets().victory_points,
        pos,
        "",
    );

    let mut tooltip = vec![];
    for (name, points) in player.victory_points_parts(rc.game) {
        tooltip.push(format!("{name}: {points}"));
    }
    show_tooltip_for_circle(
        rc,
        &tooltip,
        pos + top_center_anchor(rc) + vec2(15., 15.),
        25.,
    );

    top_icon_with_label(
        rc,
        &format!("{}", &player.incident_tokens),
        &rc.assets().event_counter,
        icon_pos(4, 0),
        "Event tokens left",
    );

    let amount = new_resource_map(&player.resources);
    let limit = new_resource_map(&player.resource_limit);
    for (i, r) in ResourceType::all().iter().rev().enumerate() {
        let a = amount[r];
        let l = limit[r];
        let t = if l > 0 {
            format!("{a}/{l}")
        } else {
            format!("{a}")
        };
        top_icon_with_label(
            rc,
            &t,
            &rc.assets().resources[r],
            icon_pos(2 - i as i8, 0),
            resource_name(*r),
        );
    }
}

pub fn show_top_left(rc: &RenderContext) {
    let state = rc.state;
    let mut p = vec2(10., 10.);
    let mut label = |label: &str| {
        multiline_label(label, 30, |label: &str| {
            p = vec2(p.x, p.y + 25.);
            if p.y > state.screen_size.y - 150. {
                p = vec2(p.x + 350., 85.);
            }
            state.draw_text(label, p.x, p.y);
        });
    };

    let game = rc.game;

    match &game.state {
        GameState::Finished => label("Finished"),
        _ => label(&format!("Age {}", game.age)),
    }
    if let Some(s) = get_status_phase(game) {
        label(&format!("Status Phase: {s}"));
    } else {
        label(&format!("Round {}", game.round));
    }

    let player = rc.shown_player;

    label(&player.get_name());

    label(&format!("Civ {}", player.civilization.name));

    label(&format!(
        "Leader {}",
        if let Some(l) = &player.active_leader() {
            &l.name
        } else {
            "-"
        }
    ));

    if game.current_player_index == player.index {
        if get_status_phase(game).is_none() && game.state != GameState::Finished {
            label(&format!("{} actions left", game.actions_left));
        }
        if let GameState::Movement(moves) = &game.state {
            let movement_actions_left = moves.movement_actions_left;
            label(&format!("Move units: {movement_actions_left} moves left"));
            match moves.current_move {
                CurrentMove::Fleet { .. } => label(
                    "May continue to move the fleet in the same sea without using movement actions",
                ),
                CurrentMove::Embark { .. } => {
                    label("May continue to embark units without using movement actions");
                }
                CurrentMove::None => {}
            }
        }
    }

    if let Some(c) = get_combat(game) {
        if c.attacker.player == player.index {
            label(&format!("Attack - combat round {}", c.round));
        } else if c.defender.player == player.index {
            label(&format!("Defend - combat round {}", c.round));
        }
    }

    if rc.shown_player_is_active() || state.active_dialog.show_for_other_player() {
        for m in state.active_dialog.help_message(rc) {
            label(&m);
        }
    }

    if rc.shown_player_is_active() {
        if let Some(u) = &state.pending_update {
            for m in &u.info {
                label(m);
            }
        }
    }

    if let Some(position) = state.focused_tile {
        show_focused_tile(&mut label, game, position);
    }

    if rc.state.show_permanent_effects {
        show_permanent_effects(rc, &mut label, game, player);
    }
}

fn show_focused_tile(label: &mut impl FnMut(&str), game: &Game, position: Position) {
    label(&format!(
        "{}/{}",
        position,
        game.map
            .get(position)
            .map_or("outside the map", terrain_name),
    ));

    if let Some(c) = game.try_get_any_city(position) {
        for l in city_labels(game, c) {
            label(&l);
        }
    }

    let units = unit_ui::units_on_tile(game, position).collect_vec();
    if !units.is_empty() {
        label(&format!("Controlled by: {}", game.player_name(units[0].0)));
    }

    for (p, unit) in units {
        let army_move = game.player(p).has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
        label(&unit_ui::unit_label(&unit, army_move, game));
    }
}

fn show_permanent_effects(
    rc: &RenderContext,
    label: &mut impl FnMut(&str),
    game: &Game,
    player: &Player,
) {
    let s = &player.secrets;
    if !s.is_empty() {
        label("Secrets:");
        for e in s {
            label(e);
        }
    }
    label("Permanent effects:");
    for e in &game.permanent_effects {
        for m in e.description(rc.game) {
            label(&m);
        }
    }
}

pub fn get_combat(game: &Game) -> Option<&CombatStats> {
    game.events.last().and_then(|e| match &e.event_type {
        PersistentEventType::CombatStart(c) => Some(&c.stats),
        PersistentEventType::CombatRoundStart(s) => Some(&s.combat.stats),
        PersistentEventType::CombatRoundEnd(e) => Some(&e.combat.stats),
        PersistentEventType::EndCombat(s) => Some(s),
        _ => None,
    })
}

pub fn show_global_controls(rc: &RenderContext, features: &Features) -> StateUpdate {
    let assets = rc.assets();
    let can_control = rc.can_control_shown_player();
    if can_control {
        let game = rc.game;
        if let Some(tooltip) = can_end_move(game) {
            if bottom_right_texture(rc, &assets.end_turn, icon_pos(-4, -1), tooltip) {
                return end_move(game);
            }
        }
        if game.can_redo() && bottom_right_texture(rc, &assets.redo, icon_pos(-5, -1), "Redo") {
            return StateUpdate::Execute(Action::Redo);
        }
        if game.can_undo() && bottom_right_texture(rc, &assets.undo, icon_pos(-6, -1), "Undo") {
            return StateUpdate::Execute(Action::Undo);
        }

        if can_control {
            let update = action_buttons(rc);
            if !matches!(update, StateUpdate::None) {
                return update;
            }
        }
    }

    if features.import_export {
        if bottom_right_texture(rc, &assets.export, icon_pos(-1, -3), "Export") {
            return StateUpdate::Export;
        }
        if bottom_right_texture(rc, &assets.import, icon_pos(-2, -3), "Import") {
            return StateUpdate::Import;
        }
    }

    if features.ai {
        let tooltip = if rc.state.ai_autoplay {
            "Pause AI autoplay"
        } else {
            "Start AI autoplay"
        };
        let assets = rc.assets();
        let texture = if rc.state.ai_autoplay {
            &assets.pause
        } else {
            &assets.play
        };
        if bottom_right_texture(rc, texture, icon_pos(-3, -3), tooltip) {
            return StateUpdate::ToggleAiPlay;
        }
    }

    StateUpdate::None
}

fn can_end_move(game: &Game) -> Option<&str> {
    if !game.events.is_empty() {
        return None;
    }
    match game.state {
        GameState::Movement(_) => Some("End movement"),
        GameState::Playing => Some("End turn"),
        GameState::Finished => None,
    }
}

fn end_move(game: &Game) -> StateUpdate {
    if let GameState::Movement(m) = &game.state {
        let movement_actions_left = m.movement_actions_left;
        return StateUpdate::execute_with_warning(
            Action::Movement(MovementAction::Stop),
            if movement_actions_left > 0 {
                vec![format!("{movement_actions_left} movement actions left")]
            } else {
                vec![]
            },
        );
    }

    let left = game.actions_left;
    StateUpdate::execute_with_warning(
        Action::Playing(PlayingAction::EndTurn),
        if left > 0 {
            vec![format!("{left} actions left")]
        } else {
            vec![]
        },
    )
}

pub fn choose_player_dialog(
    rc: &RenderContext,
    choices: &[usize],
    execute: impl Fn(usize) -> Action,
) -> StateUpdate {
    let player = rc.shown_player.index;
    if rc.can_control_active_player() && choices.contains(&player) {
        bottom_centered_text(rc, &format!("Select {}", rc.shown_player.get_name()));
        if ok_button(rc, OkTooltip::Valid("Select".to_string())) {
            return StateUpdate::execute(execute(player));
        }
    }
    StateUpdate::None
}
