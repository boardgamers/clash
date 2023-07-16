use std::collections::HashMap;
use std::f32::consts::PI;

use macroquad::hash;
use macroquad::math::{f32, i32, vec2};
use macroquad::prelude::{draw_circle, draw_text};
use macroquad::ui::root_ui;
use server::city::City;
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::resource_pile::PaymentOptions;

use crate::{ActiveDialog, map, ui};
use crate::payment::{new_resource_map, Payment, ResourcePayment, ResourceType};
use crate::ui::Point;

pub struct ConstructionPayment {
    pub player_index: usize,
    city_position: Position,
    city_piece: Building,
    payment: Payment,
}

impl ConstructionPayment {
    fn new(game: &mut Game, player_index: usize, city_position: Position, city_piece: Building) -> ConstructionPayment {
        let cost = game.players[player_index].construct_cost(&city_piece, game.players[player_index].get_city(&city_position).unwrap());
        let payment_options = game.players[player_index].resources().get_payment_options(&cost);

        let payment = ConstructionPayment::new_payment(payment_options);

        ConstructionPayment {
            player_index,
            city_position: city_position.clone(),
            city_piece: city_piece.clone(),
            payment,
        }
    }

    pub fn new_payment(a: PaymentOptions) -> Payment {
        let left = HashMap::from([
            (ResourceType::Discount, a.jokers_left),
            (ResourceType::Gold, a.gold_left),
        ]);

        let mut resources: Vec<ResourcePayment> = new_resource_map(a.default)
            .into_iter()
            .map(|e| match e.0 {
                ResourceType::Discount | ResourceType::Gold => ResourcePayment {
                    resource: e.0.clone(),
                    current: e.1,
                    min: e.1,
                    max: e.1,
                },
                _ => ResourcePayment {
                    resource: e.0.clone(),
                    current: e.1,
                    min: e.1 - a.jokers_left - a.gold_left,
                    max: e.1,
                },
            })
            .collect();

        resources.sort_by_key(|r| r.resource.clone());

        Payment {
            resources
        }
    }
}

pub fn show_city_menu(game: &mut Game, player_index: usize, city_position: &Position) -> Option<ActiveDialog> {
    let mut result = None;
    root_ui().window(hash!(), vec2(600., 20.), vec2(100., 200.), |ui| {
        for (building, name) in building_names() {
            let player = &game.players[player_index];
            let city = player.get_city(city_position).expect("city not found");
            if city.can_construct(&building, player) && ui.button(None, name) {
                result = Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(game, player_index, city_position.clone(), building)));
            };
        }
    });
    result
}

pub fn pay_construction_dialog(game: &mut Game, payment: &mut ConstructionPayment) -> bool {
    // payment_dialog()
    //
    // game.execute_action(
    //     Action::PlayingAction(PlayingAction::Construct {
    //         city_position: city_position.clone(),
    //         city_piece: building,
    //         payment: cost,
    //         temple_bonus: None,
    //     }),
    //     player_index,
    // );
    false
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
