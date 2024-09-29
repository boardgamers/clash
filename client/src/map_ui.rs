use itertools::Itertools;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::Ui;

use server::action::Action;
use server::combat::Combat;
use server::game::{Game, GameState};
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::unit::{MovementRestriction, Unit};

use crate::city_ui::{draw_city, show_city_menu, CityMenu};
use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::dialog_ui::closeable_dialog_window;
use crate::{collect_ui, hex_ui, unit_ui};

fn terrain_font_color(t: &Terrain) -> Color {
    match t {
        Terrain::Forest | Terrain::Water | Terrain::Fertile => WHITE,
        _ => BLACK,
    }
}

fn terrain_name(t: &Terrain) -> &'static str {
    match t {
        Terrain::Barren => "Barren",
        Terrain::Mountain => "Mountain",
        Terrain::Fertile => "Fertile",
        Terrain::Forest => "Forest",
        Terrain::Exhausted(_) => "Exhausted",
        Terrain::Water => "Water",
    }
}

pub fn draw_map(game: &Game, state: &State) {
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
        );
        collect_ui::draw_resource_collect_tile(state, *pos);
    }
    if let GameState::Combat(c) = &game.state {
        draw_combat_arrow(c);
    }
    if !state.is_collect() {
        for p in &game.players {
            for city in &p.cities {
                draw_city(p, city, state);
            }
        }
        unit_ui::draw_units(game);
    }
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
    draw_line(from.x, from.y, to.x, to.y, 10., BLACK);
    draw_triangle(
        vec2(to.x, to.y),
        vec2(to.x + 30., to.y + 30.),
        vec2(to.x - 30., to.y + 30.),
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

pub fn show_tile_menu(game: &Game, position: Position, player: &ShownPlayer) -> StateUpdate {
    if let Some(c) = game.get_any_city(position) {
        show_city_menu(game, &CityMenu::new(player, c.player_index, position))
    } else {
        show_generic_tile_menu(game, position, player, vec![], |_| StateUpdate::None)
    }
}

pub fn show_generic_tile_menu(
    game: &Game,
    position: Position,
    player: &ShownPlayer,
    suffix: Vec<String>,
    additional: impl FnOnce(&mut Ui) -> StateUpdate,
) -> StateUpdate {
    closeable_dialog_window(
        &format!(
            "{}/{}",
            position,
            game.map
                .tiles
                .get(&position)
                .map_or("outside the map", terrain_name),
        ),
        |ui| {
            let units: Vec<(&Unit, String)> = unit_ui::units_on_tile(game, position)
                .map(|(p, u)| {
                    let unit = game.get_player(p).get_unit(u).unwrap();
                    (unit, unit_ui::label(unit))
                })
                .collect();

            let units_str = &units.iter().map(|(_, l)| l).join(", ");
            if !units_str.is_empty() {
                ui.label(None, units_str);
            }
            for s in suffix {
                ui.label(None, &s);
            }

            let settlers = &units
                .iter()
                .filter_map(|(unit, _)| {
                    if unit.can_found_city(game) {
                        Some(unit)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if player.can_play_action && !settlers.is_empty() && ui.button(None, "Settle") {
                let settler = settlers
                    .iter()
                    .find(|u| u.movement_restriction != MovementRestriction::None)
                    .unwrap_or(&settlers[0]);
                return StateUpdate::execute(Action::Playing(PlayingAction::FoundCity {
                    settler: settler.id,
                }));
            }

            additional(ui)
        },
    )
}
