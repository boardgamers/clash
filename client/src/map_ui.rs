use itertools::Itertools;
use macroquad::hash;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};

use server::game::Game;
use server::map::Terrain;
use server::position::Position;

use crate::city_ui::draw_city;
use crate::ui_state::{ActiveDialog, State, StateUpdate, StateUpdates};

use crate::{collect_ui, hex_ui, unit_ui};

fn terrain_color(t: &Terrain) -> (Color, bool) {
    match t {
        Terrain::Barren => (Color::from_hex(0x00B2_6C19), true),
        Terrain::Mountain => (Color::from_hex(0x0057_5757), true),
        Terrain::Fertile => (Color::from_hex(0x005D_B521), false),
        Terrain::Forest => (Color::from_hex(0x0008_570D), true),
        Terrain::Exhausted => (RED, false),
        Terrain::Water => (Color::from_hex(0x001D_70F5), false),
    }
}

fn terrain_name(t: &Terrain) -> &'static str {
    match t {
        Terrain::Barren => "Barren",
        Terrain::Mountain => "Mountain",
        Terrain::Fertile => "Fertile",
        Terrain::Forest => "Forest",
        Terrain::Exhausted => "Exhausted",
        Terrain::Water => "Water",
    }
}

pub fn draw_map(game: &Game, state: &State) {
    game.map.tiles.iter().for_each(|(pos, t)| {
        let c = terrain_color(t);
        // let alpha = if state.focused_tile.iter().any(|f| *pos == f.position) {
        //     0.5
        // } else  {
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
            _ => {
                if state.focused_tile.iter().any(|f| *pos == f.position) {
                    0.5
                } else {
                    1.0
                }
            }
        };
        // };

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
        let units_str = unit_ui::units_on_tile(game, position)
            .map(|(p, u)| unit_ui::label(game.get_player(p).get_unit(u).unwrap()))
            .join(", ");
        if !units_str.is_empty() {
            ui.label(None, &units_str);
        }
        if let Some(suffix) = suffix {
            ui.label(None, suffix);
        }

        // if let ActiveDialog::MoveUnits(s) = dialog {
        //     if let Some(start) = s.start {
        //         //todo try to move here
        //     } else {
        //         let mut new = s.clone();
        //         new.start = Some(position);
        //         updates.add(StateUpdate::SetDialog(ActiveDialog::MoveUnits(new)));
        //     }
        // }

        additional(ui, &mut updates);
    });
    updates.result()
}
