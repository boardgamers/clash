use macroquad::math::vec2;
use macroquad::prelude::*;
use server::action::Action;
use server::combat::Combat;
use server::game::GameState;
use server::map::{Rotation, Terrain, UnexploredBlock};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::unit::{MovementRestriction, Unit, UnitType};
use std::collections::HashMap;
use std::ops::{Add, Mul, Rem, Sub};
use std::vec;

use crate::city_ui::{draw_city, show_city_menu, IconAction, IconActionVec};
use crate::client_state::{ActiveDialog, State, StateUpdate};
use crate::dialog_ui::{cancel_button_pos, ok_button, OkTooltip};
use crate::layout_ui::{bottom_center_texture, bottom_right_texture, icon_pos};
use crate::move_ui::{movable_units, MoveDestination, MoveIntent};
use crate::render_context::RenderContext;
use crate::{collect_ui, hex_ui, unit_ui};

#[derive(Clone)]
pub struct ExploreResolutionConfig {
    pub block: UnexploredBlock,
    pub rotation: Rotation,
}

fn terrain_font_color(t: &Terrain) -> Color {
    match t {
        Terrain::Forest | Terrain::Water | Terrain::Fertile => WHITE,
        _ => BLACK,
    }
}

pub fn terrain_name(t: &Terrain) -> &'static str {
    match t {
        Terrain::Barren => "Barren",
        Terrain::Mountain => "Mountain",
        Terrain::Fertile => "Grassland",
        Terrain::Forest => "Forest",
        Terrain::Exhausted(_) => "Exhausted",
        Terrain::Water => "Water",
        Terrain::Unexplored => "Unexplored",
    }
}

pub fn draw_map(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let overlay = get_overlay(rc);
    for (pos, t) in &game.map.tiles {
        let terrain = overlay.get(pos).unwrap_or(t);
        let (base, exhausted) = match terrain {
            Terrain::Exhausted(e) => (e.as_ref(), true),
            _ => (terrain, false),
        };

        hex_ui::draw_hex(
            *pos,
            terrain_font_color(terrain),
            alpha(rc, *pos),
            rc.assets().terrain.get(base),
            exhausted,
            rc,
        );
        let update = collect_ui::draw_resource_collect_tile(rc, *pos);
        if !matches!(update, StateUpdate::None) {
            return update;
        }
    }
    if let GameState::Combat(c) = &game.state {
        draw_combat_arrow(c);
    }
    let state = &rc.state;
    if !matches!(&state.active_dialog, ActiveDialog::CollectResources(_)) {
        for p in &game.players {
            for city in &p.cities {
                draw_city(rc, city);
            }
        }
        unit_ui::draw_units(rc, false);
        unit_ui::draw_units(rc, true);
    }
    StateUpdate::None
}

fn get_overlay(rc: &RenderContext) -> HashMap<Position, Terrain> {
    if let ActiveDialog::ExploreResolution(r) = &rc.state.active_dialog {
        r.block
            .block
            .tiles(&r.block.position, r.rotation)
            .iter()
            .map(|(pos, t)| (*pos, t.clone()))
            .collect()
    } else {
        HashMap::new()
    }
}

pub fn pan_and_zoom(state: &mut State) {
    let (_, wheel) = mouse_wheel();
    let new_zoom = state.camera.zoom + wheel * 0.0001;
    let x = new_zoom.x;
    if x < 0.005 && x > 0.0005 {
        state.camera.zoom = new_zoom;
    }

    let pan_map = is_mouse_button_down(MouseButton::Left);
    if state.pan_map && pan_map {
        let new_offset = state
            .camera
            .offset
            .add(mouse_delta_position().mul(vec2(-1., 1.)));
        if new_offset.x.abs() < 1. && new_offset.y.abs() < 1. {
            state.camera.offset = new_offset;
        }
    }
    state.pan_map = pan_map;
}

fn alpha(rc: &RenderContext, pos: Position) -> f32 {
    let game = rc.game;
    let state = &rc.state;
    let alpha = match &state.active_dialog {
        ActiveDialog::MoveUnits(s) => {
            if let Some(start) = s.start {
                if start == pos {
                    0.5
                } else if s
                    .destinations
                    .iter()
                    .any(|d| matches!(d, MoveDestination::Tile(p) if *p == pos))
                {
                    0.8
                } else {
                    0.
                }
            } else {
                0.
            }
        }
        ActiveDialog::RazeSize1City => {
            highlight_if(game.players[game.active_player()].can_raze_city(pos))
        }
        ActiveDialog::PlaceSettler => {
            highlight_if(game.players[game.active_player()].get_city(pos).is_some())
        }
        _ => {
            if let Some(p) = state.focused_tile {
                highlight_if(p == pos)
            } else {
                0.
            }
        }
    };
    alpha
}

fn draw_combat_arrow(c: &Combat) {
    let from = hex_ui::center(c.attacker_position);
    let to = hex_ui::center(c.defender_position);
    let to_vec = vec2(to.x, to.y);
    let from_vec = vec2(from.x, from.y);
    let end = from_vec.add(to_vec.sub(from_vec).mul(0.7));
    draw_line(from.x, from.y, end.x, end.y, 10., BLACK);
    let angle = from_vec.sub(to_vec).normalize();
    draw_triangle(
        to_vec.add(angle.rotate(vec2(10., 0.))),
        to_vec.add(angle.rotate(vec2(30., 30.))),
        to_vec.add(angle.rotate(vec2(30., -30.))),
        BLACK,
    );
}

fn highlight_if(b: bool) -> f32 {
    if b {
        0.5
    } else {
        0.
    }
}

pub fn show_tile_menu(rc: &RenderContext, pos: Position) -> StateUpdate {
    if let Some(city) = rc.game.get_any_city(pos) {
        if city.can_activate() && rc.shown_player.index == city.player_index {
            return show_city_menu(rc, city);
        }
    };

    let mut icons = move_units_buttons(rc, pos);
    if let Some(action) = found_city_button(rc, pos) {
        icons.push(action);
    }

    show_map_action_buttons(rc, &icons)
}

fn found_city_button<'a>(rc: &'a RenderContext<'a>, pos: Position) -> Option<IconAction<'a>> {
    if !rc.can_play_action(PlayingActionType::FoundCity) {
        return None;
    }
    let game = rc.game;
    let settlers: Vec<Unit> = unit_ui::units_on_tile(game, pos)
        .filter_map(|(_, unit)| {
            if unit.can_found_city(game) {
                Some(unit)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if settlers.is_empty() {
        None
    } else {
        Some((
            &rc.assets().units[&UnitType::Settler],
            "Found a new city".to_string(),
            Box::new(move || {
                let settler = settlers
                    .iter()
                    .find(|u| u.movement_restriction != MovementRestriction::None)
                    .unwrap_or(&settlers[0]);
                StateUpdate::execute(Action::Playing(PlayingAction::FoundCity {
                    settler: settler.id,
                }))
            }),
        ))
    }
}

pub fn move_units_button<'a>(
    rc: &'a RenderContext,
    pos: Position,
    move_intent: MoveIntent,
) -> Option<IconAction<'a>> {
    if !rc.can_play_action(PlayingActionType::MoveUnits)
        || movable_units(pos, rc.game, rc.shown_player, move_intent.to_predicate()).is_empty()
    {
        return None;
    }
    Some((
        move_intent.icon(rc),
        move_intent.toolip().to_string(),
        Box::new(move || StateUpdate::MoveUnits(move_intent.clone())),
    ))
}

pub fn move_units_buttons<'a>(rc: &'a RenderContext, pos: Position) -> Vec<IconAction<'a>> {
    let mut res = vec![];
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Land) {
        res.push(action);
    }
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Sea) {
        res.push(action);
    }
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Disembark) {
        res.push(action);
    }
    res
}

pub fn show_map_action_buttons(rc: &RenderContext, icons: &IconActionVec) -> StateUpdate {
    for (i, (icon, tooltip, action)) in icons.iter().enumerate() {
        if bottom_center_texture(
            rc,
            icon,
            icon_pos(-(icons.len() as i8) / 2 + i as i8, -1),
            tooltip,
        ) {
            return action();
        }
    }
    StateUpdate::None
}

pub fn explore_dialog(rc: &RenderContext, r: &ExploreResolutionConfig) -> StateUpdate {
    if ok_button(
        rc,
        OkTooltip::Valid("Accept current tile rotation".to_string()),
    ) {
        return StateUpdate::execute(Action::ExploreResolution(r.rotation));
    }
    if bottom_right_texture(
        rc,
        &rc.assets().rotate_explore,
        cancel_button_pos(),
        "Rotate tile",
    ) {
        let mut new = r.clone();
        new.rotation = (r.rotation + 3).rem(6);
        return StateUpdate::OpenDialog(ActiveDialog::ExploreResolution(new));
    };

    StateUpdate::None
}
