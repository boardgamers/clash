use hex2d::{Coordinate, Spacing};
use macroquad::color::{Color, DARKGRAY};
use macroquad::math::{f32, i32, vec2};
use macroquad::prelude::{
    draw_hexagon, draw_text, draw_texture, draw_texture_ex, load_texture, DrawTextureParams, Rect,
    Texture2D, BLACK, WHITE,
};
use server::map::Terrain;
use server::position::Position;
use std::f32::consts::PI;

const SIZE: f32 = 60.0;

const SHORT_SIZE: f32 = SIZE * 0.866_025_4;

const SPACING: Spacing = Spacing::FlatTop(SIZE);

pub fn center(pos: Position) -> Point {
    let c = pos.coordinate();
    let p = c.to_pixel(SPACING);
    Point { x: p.0, y: p.1 }.to_screen()
}

pub async fn draw_hex(p: Position, fill_color: Color, text_color: Color, alpha: f32, t: &Terrain) {
    let c = center(p);
    let mut v = fill_color.to_vec();
    v.w = alpha;

    let file = match t {
        Terrain::Barren => "assets/barren.png",
        Terrain::Mountain => "assets/mountain.png",
        Terrain::Fertile => "assets/grassland.png",
        Terrain::Forest => "assets/forest.png",
        Terrain::Exhausted => "assets/grassland.png", //todo
        Terrain::Water => "assets/water.png",
    };

    let texture: Texture2D = load_texture(file).await.unwrap();
    draw_texture_ex(
        &texture,
        c.x - SIZE,
        c.y - SHORT_SIZE,
        WHITE,
        DrawTextureParams {
            source: Some(Rect::new(0., 0., 298., 257.)),
            dest_size: Some(vec2(SIZE * 2.0, SHORT_SIZE * 2.)),
            ..Default::default()
        },
    );
    // draw_hexagon(c.x, c.y, SIZE, 2.0, false, DARKGRAY, Color::from_vec(v));
    draw_text(&p.to_string(), c.x - 30.0, c.y - 35.0, 20.0, text_color);
}

pub fn draw_hex_center_text(p: Position, text: &str) {
    let c = center(p);
    draw_text(text, c.x - 5., c.y + 6., 25.0, BLACK);
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
