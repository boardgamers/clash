use crate::city_ui::draw_city;
use crate::ui_state::State;
use crate::{collect_ui, hex_ui, unit_ui};
use macroquad::prelude::*;
use server::game::Game;
use server::map::Terrain;

fn terrain_color(t: &Terrain) -> (Color, bool) {
    match t {
        Terrain::Barren => (Color::from_hex(0x00B2_6C19), true),
        Terrain::Mountain => (Color::from_hex(0x0057_5757), true),
        Terrain::Fertile => (Color::from_hex(0x005D_B521), false),
        Terrain::Forest => (Color::from_hex(0x0008_570D), true),
        Terrain::Unusable => (RED, false),
        Terrain::Water => (Color::from_hex(0x001D_70F5), false),
    }
}

pub fn draw_map(game: &Game, state: &State) {
    game.map.tiles.iter().for_each(|(pos, t)| {
        let c = terrain_color(t);
        let selected = state.focused_city.into_iter().any(|(_, p)| *pos == p);
        let text_color = if c.1 { WHITE } else { BLACK };
        hex_ui::draw_hex(*pos, c.0, text_color, selected);
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
