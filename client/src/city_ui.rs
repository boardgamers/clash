use macroquad::color::BLACK;
use std::cmp;
use std::collections::HashMap;
use std::f32::consts::PI;

use macroquad::hash;
use macroquad::math::{f32, i32, vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::PaymentOptions;

use crate::map_ui::pixel_to_coordinate;
use crate::payment_ui::{
    new_resource_map, payment_dialog, HasPayment, Payment, ResourcePayment, ResourceType,
};
use crate::ui::{Point, State};
use crate::{map_ui, ui, ActiveDialog};

pub struct ConstructionPayment {
    player_index: usize,
    city_position: Position,
    city_piece: Building,
    payment: Payment,
    payment_options: PaymentOptions,
}

impl ConstructionPayment {
    fn new(
        game: &Game,
        player_index: usize,
        city_position: Position,
        city_piece: Building,
    ) -> ConstructionPayment {
        let cost = game.players[player_index].construct_cost(
            &city_piece,
            game.players[player_index].get_city(&city_position).unwrap(),
        );
        let payment_options = game.players[player_index]
            .resources()
            .get_payment_options(&cost);

        let payment = ConstructionPayment::new_payment(&payment_options);

        ConstructionPayment {
            player_index,
            city_position,
            city_piece,
            payment,
            payment_options,
        }
    }

    pub fn new_payment(a: &PaymentOptions) -> Payment {
        let mut resources: Vec<ResourcePayment> = new_resource_map(&a.default)
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
                    min: cmp::max(0, e.1 as i32 - a.jokers_left as i32 - a.gold_left as i32) as u32,
                    max: e.1,
                },
            })
            .collect();

        resources.sort_by_key(|r| r.resource.clone());

        Payment { resources }
    }
}

impl HasPayment for ConstructionPayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}

pub fn show_city_menu(
    game: &Game,
    player_index: usize,
    city_position: &Position,
) -> Option<ActiveDialog> {
    let mut result = None;
    root_ui().window(hash!(), vec2(400., 20.), vec2(100., 200.), |ui| {
        for (building, name) in building_names() {
            let player = &game.players[player_index];
            let city = player.get_city(city_position).expect("city not found");
            if game.actions_left > 0
                && city.can_construct(&building, player)
                && ui.button(None, name)
            {
                result = Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                    game,
                    player_index,
                    city_position.clone(),
                    building,
                )));
            };
        }
    });
    result
}

pub fn pay_construction_dialog(game: &mut Game, payment: &mut ConstructionPayment) -> bool {
    payment_dialog(
        payment,
        |cp| cp.payment.get(ResourceType::Discount).current == 0,
        |cp| {
            game.execute_action(
                Action::PlayingAction(PlayingAction::Construct {
                    city_position: cp.city_position.clone(),
                    city_piece: cp.city_piece.clone(),
                    payment: cp.payment.to_resource_pile(),
                    temple_bonus: None,
                }),
                cp.player_index,
            )
        },
        |ap, r| match r {
            ResourceType::Gold => ap.payment_options.gold_left > 0,
            ResourceType::Discount => ap.payment_options.jokers_left > 0,
            _ => ap.payment.get(r).max > 0,
        },
        |cp, r| {
            let gold = cp.payment.get_mut(ResourceType::Gold);
            if gold.current > 0 {
                gold.current -= 1;
            } else {
                cp.payment.get_mut(ResourceType::Discount).current += 1;
            }
            cp.payment.get_mut(r).current += 1;
        },
        |cp, r| {
            let discount = cp.payment.get_mut(ResourceType::Discount);
            if discount.current > 0 {
                discount.current -= 1;
            } else {
                cp.payment.get_mut(ResourceType::Gold).current += 1;
            }
            cp.payment.get_mut(r).current -= 1;
        },
    )
}

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    map_ui::draw_hex(&city.position);

    let c = map_ui::center(&city.position).to_screen();

    draw_circle(c.x, c.y, 15.0, ui::player_color(owner.index));

    let font_size = 25.0;
    let x = c.x - 5.;
    let y = c.y + 6.;
    if state.happiness_selection_active() {
        let steps = state
            .increase_happiness_cities
            .iter()
            .find(|(p, _)| p == &city.position)
            .expect("city not found")
            .1;
        draw_text(&format!("{}", steps), x, y, font_size, BLACK);
    } else {
        match city.mood_state {
            MoodState::Happy => draw_text("+", x, y, font_size, BLACK),
            MoodState::Neutral => {}
            MoodState::Angry => draw_text("-", x, y, font_size, BLACK),
        }
    }

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

pub fn try_city_click(game: &Game, state: &mut State) {
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();

        let c = pixel_to_coordinate(x, y);

        for p in game.players.iter() {
            for city in p.cities.iter() {
                if c == city.position.coordinate() {
                    city_click(state, p, city);
                };
            }
        }
    }
}

fn city_click(state: &mut State, player: &Player, city: &City) {
    let pos = city.position.clone();
    if state.happiness_selection_active() {
        state.increase_happiness_cities = state
            .increase_happiness_cities
            .clone()
            .into_iter()
            .map(|(p, step)| if p == pos { (p, step + 1) } else { (p, step) })
            .collect();
    } else {
        state.clear();
        state.focused_city = Some((player.index, pos));
    }
}
