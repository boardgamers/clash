use crate::action_buttons::action_buttons;
use crate::city_ui::city_labels;
use crate::client::Features;
use crate::client_state::StateUpdate;
use crate::layout_ui::{
    bottom_center_texture, bottom_right_texture, icon_pos, left_mouse_button_pressed_in_rect,
    top_center_texture, ICON_SIZE,
};
use crate::map_ui::terrain_name;
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name, resource_types};
use crate::tooltip::show_tooltip_for_rect;
use crate::unit_ui;
use macroquad::math::vec2;
use macroquad::prelude::*;
use server::action::Action;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::game::{CurrentMove, Game, GameState, MoveState};
use server::playing_actions::PlayingAction;
use server::unit::MovementAction;

pub fn player_select(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let i = game
        .players
        .iter()
        .position(|p| p.index == game.starting_player_index)
        .unwrap();
    let mut players: Vec<_> = game.players.iter().map(|p| p.index).collect();
    players.rotate_left(i);

    let size = 40.;
    let mut y = (players.len() as f32 * -size) / 2.;

    for player_index in players {
        let pl = game.get_player(player_index);
        let shown = rc.shown_player.index == pl.index;
        let screen = rc.state.screen_size;
        let pos = vec2(screen.x, screen.y / 2.0) + vec2(-size, y);

        let color = player_color(pl.index);

        let w = if shown { size + 10. } else { size };
        let x = pos.x - w + size;
        draw_rectangle(x, pos.y, w, size, color);
        draw_rectangle_lines(x, pos.y, w, size, 2.0, BLACK);
        let text = format!("{}", pl.victory_points());

        let state = rc.state;
        state.draw_text(&text, pos.x + 10., pos.y + 22.);

        if game.active_player() == pl.index {
            draw_texture_ex(
                &rc.assets().active_player,
                x - 25.,
                pos.y + 10.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20., 20.)),
                    ..Default::default()
                },
            );
        }

        let rect = Rect::new(x, pos.y, w, size);
        let tooltip = if state.control_player.is_some_and(|p| p == pl.index) {
            format!("{} (You)", pl.get_name())
        } else {
            pl.get_name()
        };
        show_tooltip_for_rect(rc, &[tooltip], rect);
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

    top_icon_with_label(
        rc,
        &format!("{}", &player.victory_points()),
        &rc.assets().victory_points,
        icon_pos(3, 0),
        "Victory Points",
    );
    let amount = new_resource_map(&player.resources);
    let limit = new_resource_map(&player.resource_limit);
    for (i, r) in resource_types().iter().rev().enumerate() {
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
        p = vec2(p.x, p.y + 25.);
        if p.y > state.screen_size.y - 150. {
            p = vec2(p.x + 350., 85.);
        }
        state.draw_text(label, p.x, p.y);
    };

    let game = rc.game;

    match &game.state {
        GameState::Finished => label("Finished"),
        _ => label(&format!("Age {}", game.age)),
    }
    match &game.state {
        GameState::StatusPhase(ref p) => label(&format!("Status Phase: {p:?}")),
        _ => label(&format!("Round {}", game.round)),
    }

    let player = rc.shown_player;

    label(&player.get_name());

    label(&format!("Civ {}", player.civilization.name));

    label(&format!(
        "Leader {}",
        if let Some(l) = &player.active_leader {
            &l.name
        } else {
            "-"
        }
    ));

    if game.current_player_index == player.index {
        match &game.state {
            GameState::StatusPhase(_) | GameState::Finished => {}
            _ => label(&format!("{} actions left", game.actions_left)),
        }
        if let Some(moves) = move_state(&game.state) {
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

    if let GameState::Combat(c) = &game.state {
        if c.attacker == player.index {
            label(&format!("Attack - combat round {}", c.round));
        } else if c.defender == player.index {
            label(&format!("Defend - combat round {}", c.round));
        }
    }

    if rc.shown_player_is_active() || state.active_dialog.show_for_other_player() {
        for m in state.active_dialog.help_message(game) {
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
        label(&format!(
            "{}/{}",
            position,
            game.map
                .tiles
                .get(&position)
                .map_or("outside the map", terrain_name),
        ));

        if let Some(c) = game.get_any_city(position) {
            for l in city_labels(game, c) {
                label(&l);
            }
        }

        for (p, unit) in unit_ui::units_on_tile(game, position) {
            let army_move = game
                .get_player(p)
                .has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
            label(&unit_ui::unit_label(&unit, army_move));
        }
    }
}

fn move_state(state: &GameState) -> Option<&MoveState> {
    match state {
        GameState::Combat(c) => move_state(&c.initiation),
        GameState::ExploreResolution(r) => Some(&r.move_state),
        GameState::Movement(m) => Some(m),
        GameState::PlaceSettler(p) => Some(&p.move_state),
        _ => None,
    }
}

pub fn show_global_controls(rc: &RenderContext, features: &Features) -> StateUpdate {
    let assets = rc.assets();
    let can_control = rc.can_control();
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
        };
        if bottom_right_texture(rc, &assets.import, icon_pos(-2, -3), "Import") {
            return StateUpdate::Import;
        };
    }

    StateUpdate::None
}

fn can_end_move(game: &Game) -> Option<&str> {
    match game.state {
        GameState::Movement { .. } => Some("End movement"),
        GameState::Playing => Some("End turn"),
        _ => None,
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

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => YELLOW,
        1 => PINK,
        _ => panic!("unexpected player index"),
    }
}
