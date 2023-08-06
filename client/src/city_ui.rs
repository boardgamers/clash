use macroquad::color::BLACK;
use macroquad::hash;
use macroquad::math::{i32, u32, vec2};
use macroquad::prelude::*;
use macroquad::ui::root_ui;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::resource_pile::PaymentOptions;
use std::cmp;
use std::collections::HashMap;

use crate::construct_ui::add_construct_button;
use crate::happiness_ui::increase_happiness_click;
use crate::hex_ui::pixel_to_coordinate;
use crate::payment_ui::{new_resource_map, HasPayment, Payment, ResourcePayment, ResourceType};
use crate::ui::{can_play_action, State};
use crate::{hex_ui, influence_ui, ui, ActiveDialog};

pub struct CityMenu<'a> {
    pub player_index: usize,
    pub city_owner_index: usize,
    pub city_position: &'a Position,
}

impl<'a> CityMenu<'a> {
    pub fn new(player_index: usize, city_owner_index: usize, city_position: &'a Position) -> Self {
        CityMenu {
            player_index,
            city_owner_index,
            city_position,
        }
    }

    pub fn get_player(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.player_index)
    }

    pub fn get_city_owner(&self, game: &'a Game) -> &Player {
        game.get_player(self.city_owner_index)
    }

    pub fn get_city(&self, game: &'a Game) -> &City {
        return game.players[self.city_owner_index]
            .get_city(self.city_position)
            .expect("city not found");
    }

    pub fn is_city_owner(&self) -> bool {
        self.player_index == self.city_owner_index
    }
}

pub struct ConstructionPayment {
    pub player_index: usize,
    pub city_position: Position,
    pub city_piece: Building,
    pub payment: Payment,
    pub payment_options: PaymentOptions,
}

impl ConstructionPayment {
    pub fn new(
        game: &Game,
        player_index: usize,
        city_position: Position,
        city_piece: Building,
    ) -> ConstructionPayment {
        let p = game.get_player(player_index);
        let cost = p.construct_cost(&city_piece, p.get_city(&city_position).unwrap());
        let payment_options = p.resources().get_payment_options(&cost);

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
                    min: cmp::max(0, e.1 as i32 - a.discount as i32 - a.gold_left as i32) as u32,
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

pub fn show_city_menu(game: &mut Game, menu: CityMenu) -> Option<ActiveDialog> {
    let mut result: Option<ActiveDialog> = None;

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        ui.label(None, &menu.city_position.to_string());
        let closest_city_pos = &influence_ui::closest_city(&game, &menu);

        for (building, name) in building_names() {
            if can_play_action(game) {
                if let Some(d) = add_construct_button(game, &menu, ui, &building, name) {
                    let _ = result.insert(d);
                }

                influence_ui::add_influence_button(
                    game,
                    &menu,
                    ui,
                    closest_city_pos,
                    &building,
                    name,
                );
            };
        }
    });
    result
}

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    let c = hex_ui::center(&city.position).to_screen();

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
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
            let p = hex_ui::rotate_around(c, 30.0, 90 * i);
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
