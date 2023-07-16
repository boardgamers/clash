use server::game::{Action, Game};
use server::position::Position;
use macroquad::ui::root_ui;
use macroquad::hash;
use macroquad::math::{f32, i32, vec2};
use server::playing_actions::PlayingAction;
use server::player::Player;
use server::city::City;
use macroquad::prelude::{draw_circle, draw_text};
use std::collections::HashMap;
use server::city_pieces::Building;
use std::f32::consts::PI;
use crate::{map, ui};
use crate::ui::Point;

pub fn show_city_menu(game: &mut Game, player_index: usize, city_position: &Position) {
    root_ui().window(hash!(), vec2(600., 20.), vec2(100., 200.), |ui| {
        for (building, name) in building_names() {
            let player = &game.players[player_index];
            let city = player.get_city(city_position).expect("city not found");
            if city.can_construct(&building, player) && ui.button(None, name) {
                let cost = player.construct_cost(&building, city);
                game.execute_action(
                    Action::PlayingAction(PlayingAction::Construct {
                        city_position: city_position.clone(),
                        city_piece: building,
                        payment: cost,
                        temple_bonus: None,
                    }),
                    player_index,
                );
            };
        }
    });
}

pub fn draw_city(owner: &Player, city: &City) {
    map::draw_hex(&city.position);

    let c = map::center(&city.position).to_screen();

    draw_circle(c.x, c.y, 10.0, ui::player_color(owner.index));

    let mut i = 0;
    for player_index in 0..4 {
        for b in city.city_pieces.buildings(Some(player_index)).iter() {
            let p = rotate_around(c, 30.0, 90 * i);
            draw_text(
                building_symbol(b),
                p.x - 12.0,
                p.y + 12.0,
                50.0,
                ui::player_color(player_index),
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
