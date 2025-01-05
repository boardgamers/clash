use macroquad::math::vec2;
use macroquad::prelude::*;
use std::ops::{Add, Mul, Sub};

use server::action::Action;
use server::combat::Combat;
use server::game::{Game, GameState};
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::unit::{MovementRestriction, Unit};

use crate::city_ui::{draw_city, show_city_menu, CityMenu, IconAction, IconActionVec};
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::layout_ui::{bottom_center_texture, icon_pos};
use crate::move_ui::movable_units;
use crate::{collect_ui, hex_ui, unit_ui};

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
        Terrain::Fertile => "Fertile",
        Terrain::Forest => "Forest",
        Terrain::Exhausted(_) => "Exhausted",
        Terrain::Water => "Water",
    }
}

pub fn draw_map(game: &Game, state: &mut State) -> StateUpdate {
    pan_and_zoom(state);
    state.set_world_camera();
    for (pos, t) in &game.map.tiles {
        let (base, exhausted) = match t {
            Terrain::Exhausted(e) => (e.as_ref(), true),
            _ => (t, false),
        };

        hex_ui::draw_hex(
            *pos,
            terrain_font_color(t),
            alpha(game, state, *pos),
            state.assets.terrain.get(base).unwrap(),
            exhausted,
            state,
        );
        let update = collect_ui::draw_resource_collect_tile(state, *pos);
        if !matches!(update, StateUpdate::None) {
            return update;
        }
    }
    if let GameState::Combat(c) = &game.state {
        draw_combat_arrow(c);
    }
    if !matches!(&state.active_dialog, ActiveDialog::CollectResources(_)) {
        for p in &game.players {
            for city in &p.cities {
                draw_city(p, city, state);
            }
        }
        unit_ui::draw_units(game, state, false);
        unit_ui::draw_units(game, state, true);
    }
    state.set_screen_camera();
    StateUpdate::None
}

fn pan_and_zoom(state: &mut State) {
    let (_, wheel) = mouse_wheel();
    state.camera.zoom += wheel * 0.0001;

    let pan_map = is_mouse_button_down(MouseButton::Left);
    if state.pan_map && pan_map {
        let offset = mouse_delta_position().mul(vec2(-1., 1.));
        state.camera.offset = state.camera.offset.add(offset);
    }
    state.pan_map = pan_map;
}

fn alpha(game: &Game, state: &State, pos: Position) -> f32 {
    let alpha = match &state.active_dialog {
        ActiveDialog::MoveUnits(s) => {
            if let Some(start) = s.start {
                if start == pos {
                    0.5
                } else if s.destinations.contains(&pos) {
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

pub fn show_tile_menu<'a>(
    game: &'a Game,
    pos: Position,
    player: &'a ShownPlayer,
    state: &'a State,
) -> StateUpdate {
    if let Some(c) = game.get_any_city(pos) {
        return show_city_menu(game, &CityMenu::new(player, c.player_index, pos), state);
    };

    let settlers: Vec<Unit> = unit_ui::units_on_tile(game, pos)
        .filter_map(|(_, unit)| {
            if unit.can_found_city(game) {
                Some(unit)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut icons: IconActionVec<'a> = vec![];
    if let Some(i) = move_units_button(game, pos, player, state) {
        icons.push(i)
    }
    if let Some(i) = found_city_button(state, settlers) {
        icons.push(i)
    }

    show_map_action_buttons(state, &icons)
}

fn found_city_button(state: &State, settlers: Vec<Unit>) -> Option<IconAction<'_>> {
    if settlers.is_empty() {
        None
    } else {
        Some((
            &state.assets.settle,
            "Settle".to_string(),
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
    game: &'a Game,
    pos: Position,
    player: &'a ShownPlayer,
    state: &'a State,
) -> Option<IconAction<'a>> {
    if movable_units(pos, game, player.get(game)).is_empty() {
        return None;
    }
    Some((
        &state.assets.move_units,
        "Move units".to_string(),
        Box::new(move || StateUpdate::execute(Action::Playing(PlayingAction::MoveUnits))),
    ))
}

pub fn show_map_action_buttons(state: &State, icons: &IconActionVec) -> StateUpdate {
    for (i, (icon, tooltip, action)) in icons.iter().enumerate() {
        if bottom_center_texture(
            state,
            icon,
            icon_pos(-(icons.len() as i8) / 2 + i as i8, -1),
            tooltip,
        ) {
            return action();
        }
    }
    StateUpdate::None
}
