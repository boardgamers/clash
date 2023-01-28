use crate::ui::Point;
use hex2d::Coordinate;
use hex2d::Spacing;
use macroquad::prelude::*;
use server::hexagon::Position;
use std::f32::consts::PI;

const SIZE: f32 = 80.0;
const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: &Position) -> Point {
    let p = pos.coordinate().to_pixel(SPACING);
    Point { x: p.0, y: p.1 }
}

pub(crate) fn draw_hex(p: &Position) {
    let c = center(p).to_screen();
    draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, BLUE);
    draw_text(&p.to_string(), c.x, c.y, 40.0, DARKGRAY);
}

pub fn pixel_to_coordinate(x: f32, y: f32) -> Coordinate {
    let p = Point::new(x, y).to_game();

    let c = Coordinate::from_pixel(p.x, p.y, SPACING);
    c
}
