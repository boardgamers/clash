use crate::ui::Point;
use hex2d::{Coordinate, Spacing};
use macroquad::color::{Color, DARKGRAY};
use macroquad::math::f32;
use macroquad::prelude::{draw_hexagon, draw_text};
use server::position::Position;

const SIZE: f32 = 60.0;
const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: &Position) -> Point {
    let c = pos.coordinate();
    let p = c.to_pixel(SPACING);
    Point { x: p.0, y: p.1 }
}

pub fn draw_hex(p: &Position, fill_color: Color, text_color: Color) {
    let c = center(p).to_screen();
    draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, fill_color);
    draw_text(&p.to_string(), c.x - 30.0, c.y - 35.0, 20.0, text_color);
}

pub fn pixel_to_coordinate(x: f32, y: f32) -> Coordinate {
    let p = Point::new(x, y).to_game();
    Coordinate::from_pixel(p.x, p.y, SPACING)
}
