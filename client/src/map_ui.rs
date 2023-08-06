use crate::city_ui::draw_city;
use crate::hex_ui;
use crate::ui_state::State;
use macroquad::prelude::*;
use server::game::Game;
use server::map::Terrain;

fn terrain_color(t: &Terrain) -> (Color, bool) {
    match t {
        Terrain::Barren => (Color::from_hex(0xB26C19), true),
        Terrain::Mountain => (Color::from_hex(0x575757), true),
        Terrain::Fertile => (Color::from_hex(0x5DB521), false),
        Terrain::Forest => (Color::from_hex(0x08570D), true),
        Terrain::Unusable => (RED, false),
        Terrain::Water => (Color::from_hex(0x1D70F5), false),
    }
}

pub fn draw_map(game: &Game, state: &State) {
    game.map.tiles.iter().for_each(|(pos, t)| {
        let c = terrain_color(t);
        let selected = state.focused_city.iter().any(|(_, p)| pos == p);
        hex_ui::draw_hex(pos, c.0, if c.1 { WHITE } else { BLACK }, selected)
    });
    for p in game.players.iter() {
        for city in p.cities.iter() {
            draw_city(p, city, state);
        }
    }
}
