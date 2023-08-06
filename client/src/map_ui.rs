use crate::city_ui::draw_city;
use crate::hex_ui;
use crate::ui::State;
use macroquad::prelude::*;
use server::game::Game;
use server::map::Terrain;

fn terrain_color(t: &Terrain) -> (Color, bool) {
    match t {
        Terrain::Barren => (BROWN, true),
        Terrain::Mountain => (RED, true),
        Terrain::Fertile => (LIME, false),
        Terrain::Forest => (DARKGREEN, true),
        Terrain::Unusable => (BLACK, true),
        Terrain::Water => (BLUE, true),
    }
}

pub fn draw_map(game: &Game, state: &State) {
    game.map.tiles.iter().for_each(|(p, t)| {
        let c = terrain_color(t);
        hex_ui::draw_hex(p, c.0, if c.1 { WHITE } else { BLACK })
    });
    for p in game.players.iter() {
        for city in p.cities.iter() {
            draw_city(p, city, state);
        }
    }
}
