use crate::ui::{player_color, Point};
use hex2d::Coordinate;
use hex2d::Spacing;
use macroquad::prelude::*;
use server::city::{Building, City};
use server::hexagon::Position;
use server::player::Player;
use std::collections::HashMap;
use std::f32::consts::PI;

const SIZE: f32 = 60.0;
const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: &Position) -> Point {
    let c = pos.coordinate();
    let p = c.to_pixel(SPACING);
    Point { x: p.0, y: p.1 }
}

pub fn draw_city(owner: &Player, city: &City) {
    draw_hex(&city.position);

    let c = center(&city.position).to_screen();

    draw_circle(c.x, c.y, 10.0, player_color(owner.index));

    let mut i = 0;
    for player_index in 0..4 {
        for b in city.city_pieces.buildings(Some(player_index)).iter() {
            let p = rotate_around(c, 30.0, 90 * i);
            draw_text(
                building_symbol(b),
                p.x - 12.0,
                p.y + 12.0,
                50.0,
                player_color(player_index),
            );
            i += 1;
        }
    }
}

fn rotate_around(center: Point, radius: f32, angle_deg: i32) -> Point {
    let angle_rad = PI / 180.0 * (angle_deg as f32);
    Point {
        x: center.x + radius * f32::cos(angle_rad),
        y: center.y + radius * f32::sin(angle_rad),
    }
}

fn building_symbol(b: &Building) -> &str {
    match b {
        Building::Academy => "A",
        Building::Market => "M",
        Building::Obelisk => "K",
        Building::Observatory => "V",
        Building::Fortress => "F",
        Building::Port => "P",
        Building::Temple => "T",
    }
}

pub fn building_names() -> HashMap<Building, &'static str> {
    HashMap::from([
        (Building::Academy, "Academy"),
        (Building::Market, "Market"),
        (Building::Obelisk, "Obelisk"),
        (Building::Observatory, "Observatory"),
        (Building::Fortress, "Fortress"),
        (Building::Port, "Port"),
        (Building::Temple, "Temple"),
    ])
}

pub fn draw_hex(p: &Position) {
    let c = center(p).to_screen();
    draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, WHITE);
    draw_text(&p.to_string(), c.x - 30.0, c.y - 35.0, 20.0, DARKGRAY);
}

pub fn pixel_to_coordinate(x: f32, y: f32) -> Coordinate {
    let p = Point::new(x, y).to_game();
    Coordinate::from_pixel(p.x, p.y, SPACING)
}
