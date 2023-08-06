use macroquad::color::BLACK;
use std::cmp;
use std::collections::HashMap;
use std::f32::consts::PI;

use macroquad::hash;
use macroquad::math::{f32, i32, u32, vec2};
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::{PaymentOptions, ResourcePile};

use crate::hex_ui::pixel_to_coordinate;
use crate::payment_ui::{
    new_resource_map, payment_dialog, HasPayment, Payment, ResourcePayment, ResourceType,
};
use crate::ui::{can_play_action, IncreaseHappiness, Point, State};
use crate::{hex_ui, ui, ActiveDialog};

pub struct CityMenu<'a> {
    player_index: usize,
    city_owner_index: usize,
    city_position: &'a Position,
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

pub fn show_city_menu(game: &mut Game, menu: CityMenu) -> Option<ActiveDialog> {
    let mut result: Option<ActiveDialog> = None;

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        let closet_city_pos = &menu
            .get_player(game)
            .cities
            .iter()
            .min_by_key(|c| c.position.distance(menu.city_position))
            .unwrap()
            .position
            .clone();

        for (building, name) in building_names() {
            if can_play_action(game) {
                if let Some(d) = add_construct_button(game, &menu, ui, &building, name) {
                    let _ = result.insert(d);
                }

                add_influence_button(game, &menu, ui, closet_city_pos, &building, name);
            };
        }
    });
    result
}

fn add_construct_button(
    game: &Game,
    menu: &CityMenu,
    ui: &mut Ui,
    building: &Building,
    name: &str,
) -> Option<ActiveDialog> {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    if (menu.is_city_owner()) && city.can_construct(building, owner) && ui.button(None, name) {
        return Some(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
            game,
            menu.player_index,
            menu.city_position.clone(),
            building.clone(),
        )));
    }
    None
}

fn add_influence_button(
    game: &mut Game,
    menu: &CityMenu,
    ui: &mut Ui,
    closet_city_pos: &Position,
    building: &Building,
    building_name: &str,
) {
    let city = menu.get_city(game);

    if !city.city_pieces.can_add_building(building) {
        let start_position = if menu.is_city_owner() {
            menu.city_position
        } else {
            closet_city_pos
        };
        if let Some(cost) = game.influence_culture_boost_cost(
            menu.player_index,
            start_position,
            menu.city_owner_index,
            menu.city_position,
            building,
        ) {
            if ui.button(
                None,
                format!("Attempt Influence {} for {}", building_name, cost),
            ) {
                game.execute_action(
                    Action::PlayingAction(PlayingAction::InfluenceCultureAttempt {
                        starting_city_position: start_position.clone(),
                        target_player_index: menu.city_owner_index,
                        target_city_position: menu.city_position.clone(),
                        city_piece: building.clone(),
                    }),
                    menu.player_index,
                );
            }
        }
    }
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
    let c = hex_ui::center(&city.position).to_screen();

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
