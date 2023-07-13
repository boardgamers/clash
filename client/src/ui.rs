use macroquad::prelude::*;

const TOP_BORDER: f32 = 130.0;
const LEFT_BORDER: f32 = 90.0;

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

pub fn player_color(player_index: usize) -> Color {
    match player_index {
        0 => RED,
        1 => BLUE,
        2 => YELLOW,
        3 => BLACK,
        _ => panic!("unexpected player index"),
    }
}
