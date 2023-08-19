use hex2d::{Coordinate, Spacing};
use macroquad::color::{Color, DARKGRAY};
use macroquad::math::{f32, i32};
use macroquad::prelude::{draw_hexagon, draw_text, BLACK};
use server::position::Position;
use std::f32::consts::PI;

const SIZE: f32 = 60.0;
const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: Position) -> Point {
    let c = pos.coordinate();
    let p = c.to_pixel(SPACING);
    Point { x: p.0, y: p.1 }
}

pub fn draw_hex(p: Position, fill_color: Color, text_color: Color, selected: bool) {
    let c = center(p).to_screen();
    let mut v = fill_color.to_vec();
    if selected {
        v.w = 0.5;
    }

    draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, Color::from_vec(v));
    draw_text(&p.to_string(), c.x - 30.0, c.y - 35.0, 20.0, text_color);
}

pub fn draw_hex_center_text(p: Position, text: &str) {
    let c = center(p).to_screen();
    draw_text(text, c.x - 5., c.y + 6., 25.0, BLACK)
}

pub fn pixel_to_coordinate(x: f32, y: f32) -> Coordinate {
    let p = Point::new(x, y).to_game();
    Coordinate::from_pixel(p.x, p.y, SPACING)
}

pub fn rotate_around(center: Point, radius: f32, angle_deg: i32) -> Point {
    let angle_rad = PI / 180.0 * (angle_deg as f32);
    Point {
        x: center.x + radius * f32::cos(angle_rad),
        y: center.y + radius * f32::sin(angle_rad),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    pub fn to_screen(self) -> Point {
        let x = self.x + LEFT_BORDER;
        let y = TOP_BORDER - self.y;
        Point { x, y }
    }

    pub fn to_game(self) -> Point {
        let x = self.x - LEFT_BORDER;
        let y = TOP_BORDER - self.y;
        Point { x, y }
    }
}

const TOP_BORDER: f32 = 130.0;
const LEFT_BORDER: f32 = 90.0;
