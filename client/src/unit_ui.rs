use crate::{hex_ui, player_ui};
use macroquad::math::u32;
use macroquad::prelude::draw_text;
use server::game::Game;
use server::position::Position;
use server::unit::{Unit, UnitType};
use std::collections::HashMap;

pub fn draw_unit(unit: &Unit, index: u32) {
    let c = hex_ui::center(unit.position);
    let r = if unit.unit_type == UnitType::Settler {
        25.
    } else {
        40.
    };
    let p = hex_ui::rotate_around(c, r, (90 * index) as i32 + 45);
    draw_text(
        unit_symbol(unit),
        p.x - 7.0,
        p.y + 7.0,
        25.0,
        player_ui::player_color(unit.player_index),
    );
}

fn unit_symbol(unit: &Unit) -> &str {
    match unit.unit_type {
        UnitType::Infantry => "I",
        UnitType::Cavalry => "C",
        UnitType::Elephant => "E",
        UnitType::Leader => "L",
        UnitType::Ship => "P",
        UnitType::Settler => "S",
    }
}

pub fn draw_units(game: &Game) {
    for p in &game.players {
        let mut city_unit_index: HashMap<Position, u32> = HashMap::new();
        let mut settler_index: HashMap<Position, u32> = HashMap::new();
        for unit in &p.units {
            let map = if unit.unit_type == UnitType::Settler {
                &mut settler_index
            } else {
                &mut city_unit_index
            };
            let e = map.entry(unit.position).or_default();
            *e += 1;
            draw_unit(unit, *e);
        }
    }
}
