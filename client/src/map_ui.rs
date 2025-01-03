use macroquad::math::vec2;
use macroquad::prelude::*;
use std::ops::{Add, Mul, Sub};

use server::action::Action;
use server::combat::Combat;
use server::game::{Game, GameState};
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::unit::MovementRestriction;

use crate::city_ui::{draw_city, show_city_menu, CityMenu};
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::layout_ui::{bottom_center_texture, icon_pos};
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
        ActiveDialog::ReplaceUnits(s) => highlight_if(s.current_city.is_some_and(|p| p == pos)),
        ActiveDialog::RazeSize1City => {
            highlight_if(game.players[game.active_player()].can_raze_city(pos))
        }
        ActiveDialog::PlaceSettler => {
            highlight_if(game.players[game.active_player()].get_city(pos).is_some())
        }
        ActiveDialog::TileMenu(p) => highlight_if(*p == pos),
        _ => 0.,
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

pub fn show_tile_menu(
    game: &Game,
    position: Position,
    player: &ShownPlayer,
    state: &State,
) -> StateUpdate {
    if player.can_play_action {
        if let Some(c) = game.get_any_city(position) {
            show_city_menu(
                game,
                &CityMenu::new(player, c.player_index, position),
                state,
            )
        } else {
            let units = unit_ui::units_on_tile(game, position);
            let settlers = units
                .filter_map(|(_, unit)| {
                    if unit.can_found_city(game) {
                        Some(unit)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if !settlers.is_empty()
                && bottom_center_texture(state, &state.assets.settle, icon_pos(0, -1), "Settle")
            {
                let settler = settlers
                    .iter()
                    .find(|u| u.movement_restriction != MovementRestriction::None)
                    .unwrap_or(&settlers[0]);
                StateUpdate::execute(Action::Playing(PlayingAction::FoundCity {
                    settler: settler.id,
                }))
            } else {
                StateUpdate::None
            }
        }
    } else {
        StateUpdate::None
    }
}
