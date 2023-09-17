use itertools::Itertools;
use macroquad::hash;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::action::Action;

use server::game::Game;
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::unit::{MovementRestriction, Unit};

use crate::city_ui::draw_city;
use crate::ui_state::{can_play_action, ActiveDialog, State, StateUpdate, StateUpdates};

use crate::{collect_ui, hex_ui, unit_ui};

fn terrain_color(t: &Terrain) -> (Color, bool) {
    match t {
        Terrain::Barren => (Color::from_hex(0x00B2_6C19), true),
        Terrain::Mountain => (Color::from_hex(0x0057_5757), true),
        Terrain::Fertile => (Color::from_hex(0x005D_B521), false),
        Terrain::Forest => (Color::from_hex(0x0008_570D), true),
        Terrain::Exhausted(_) => (RED, false),
        Terrain::Water => (Color::from_hex(0x001D_70F5), false),
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
    game.map.tiles.iter().for_each(|(pos, t)| {
        let c = terrain_color(t);
        let alpha = match &state.active_dialog {
            ActiveDialog::MoveUnits(s) => {
                if let Some(start) = s.start {
                    if start == *pos {
                        0.5
                    } else if s.destinations.contains(pos) {
                        0.2
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            }
            ActiveDialog::ReplaceUnits(s) => {
                highlight_if(s.current_city.is_some_and(|p| p == *pos))
            }
            ActiveDialog::RaseSize1City => {
                highlight_if(game.players[game.active_player()].can_raze_city(*pos))
            }
            ActiveDialog::PlaceSettler => {
                highlight_if(game.players[game.active_player()].get_city(*pos).is_some())
            }
            _ => highlight_if(
                state
                    .focused_tile
                    .as_ref()
                    .is_some_and(|f| pos == &f.position),
            ),
        };

        let text_color = if c.1 { WHITE } else { BLACK };
        hex_ui::draw_hex(*pos, c.0, text_color, alpha);
        collect_ui::draw_resource_collect_tile(state, *pos);
    });
    if !state.is_collect() {
        for p in &game.players {
            for city in &p.cities {
                draw_city(p, city, state);
            }
        }
        unit_ui::draw_units(game);
    }
}

fn highlight_if(b: bool) -> f32 {
    if b {
        0.5
    } else {
        1.0
    }
}

pub fn show_tile_menu(
    game: &Game,
    position: Position,
    suffix: Option<&str>,
    additional: impl FnOnce(&mut Ui, &mut StateUpdates),
) -> StateUpdate {
    let mut updates: StateUpdates = StateUpdates::new();

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        ui.label(
            None,
            &format!(
                "{}/{}",
                position,
                game.map
                    .tiles
                    .get(&position)
                    .map_or("outside the map", terrain_name),
            ),
        );
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
        if let Some(suffix) = suffix {
            ui.label(None, suffix);
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

        if can_play_action(game) && !settlers.is_empty() && ui.button(None, "Settle") {
            let settler = settlers
                .iter()
                .find(|u| u.movement_restriction != MovementRestriction::None)
                .unwrap_or(&settlers[0]);
            updates.add(StateUpdate::execute(Action::Playing(
                PlayingAction::FoundCity {
                    settler: settler.id,
                },
            )));
        }

        additional(ui, &mut updates);
    });
    updates.result()
}
