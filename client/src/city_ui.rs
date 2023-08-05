use macroquad::color::BLACK;
use std::cmp;
use std::collections::HashMap;
use std::f32::consts::PI;

use macroquad::hash;
use macroquad::math::{f32, i32, u32, vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::{PaymentOptions, ResourcePile};

use crate::map_ui::pixel_to_coordinate;
use crate::payment_ui::{
    new_resource_map, payment_dialog, HasPayment, Payment, ResourcePayment, ResourceType,
};
use crate::ui::{can_play_action, IncreaseHappiness, Point, State};
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
    game: &mut Game,
    player_index: usize,
    city_owner_index: usize,
    city_position: &Position,
) -> Option<ActiveDialog> {
    let mut result = None;

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        let closet_city_pos = &game.players[player_index]
            .cities
            .iter()
            .min_by_key(|c| c.position.distance(city_position))
            .unwrap()
            .position
            .clone();

        for (building, name) in building_names() {
            let owner = &game.players[city_owner_index];
            let is_owner = player_index == city_owner_index;
            let city = owner.get_city(city_position).expect("city not found");
            if can_play_action(game) {
                if is_owner && city.can_construct(&building, owner) && ui.button(None, name) {
                    result = Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                        game,
                        player_index,
                        city_position.clone(),
                        building.clone(),
                    )));
                }
                if !city.city_pieces.can_add_building(&building) {
                    let start_position = if is_owner {
                        city_position
                    } else {
                        closet_city_pos
                    };
                    if let Some(cost) = game.influence_culture_boost_cost(
                        player_index,
                        start_position,
                        city_owner_index,
                        city_position,
                        &building,
                    ) {
                        if ui.button(None, format!("Attempt Influence {} for {}", name, cost)) {
                            game.execute_action(
                                Action::PlayingAction(PlayingAction::InfluenceCultureAttempt {
                                    starting_city_position: start_position.clone(),
                                    target_player_index: city_owner_index,
                                    target_city_position: city_position.clone(),
                                    city_piece: building.clone(),
                                }),
                                player_index,
                            );
                        }
                    }
                }
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
    if let Some(increase) = &state.increase_happiness {
        let steps = increase
            .steps
            .iter()
            .find(|(p, _)| p == &city.position)
            .map_or(String::new(), |(_, s)| format!("{}", s));
        draw_text(&steps, x, y, font_size, BLACK);
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
    let pos = &city.position;

    if let Some(increase_happiness) = &state.increase_happiness {
        state.increase_happiness = Some(increase_happiness_click(
            player,
            city,
            pos,
            increase_happiness,
        ))
    } else {
        state.clear();
        state.focused_city = Some((player.index, pos.clone()));
    }
}

fn increase_happiness_click(
    player: &Player,
    city: &City,
    pos: &Position,
    increase_happiness: &IncreaseHappiness,
) -> IncreaseHappiness {
    let mut total_cost = increase_happiness.cost.clone();
    let new_steps = increase_happiness
        .steps
        .iter()
        .map(|(p, steps)| {
            let old_steps = *steps;
            if p == pos {
                if let Some(r) = increase_happiness_steps(player, city, &total_cost, old_steps) {
                    total_cost = r.1;
                    return (p.clone(), r.0);
                };
            }
            (p.clone(), old_steps)
        })
        .collect();

    IncreaseHappiness::new(new_steps, total_cost)
}

fn increase_happiness_steps(
    player: &Player,
    city: &City,
    total_cost: &ResourcePile,
    old_steps: u32,
) -> Option<(u32, ResourcePile)> {
    if let Some(value) =
        increase_happiness_new_steps(player, city, total_cost, old_steps, old_steps + 1)
    {
        return Some(value);
    }
    if let Some(value) = increase_happiness_new_steps(player, city, total_cost, old_steps, 0) {
        return Some(value);
    }
    None
}

fn increase_happiness_new_steps(
    player: &Player,
    city: &City,
    total_cost: &ResourcePile,
    old_steps: u32,
    new_steps: u32,
) -> Option<(u32, ResourcePile)> {
    if let Some(new_cost) = city.increase_happiness_cost(new_steps) {
        let mut new_total = total_cost.clone();
        if old_steps > 0 {
            new_total -= city
                .increase_happiness_cost(old_steps)
                .expect("invalid steps");
        }
        new_total += new_cost;
        if player.resources().can_afford(&new_total) {
            return Some((new_steps, new_total));
        }
    }
    None
}
