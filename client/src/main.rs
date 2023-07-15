extern crate core;

use std::fmt::Debug;

use macroquad::hash;
use macroquad::prelude::*;
use macroquad::ui::root_ui;
use macroquad::ui::widgets::Group;
use server::city::City;
use server::content::advances::get_technologies;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::map::{building_names, pixel_to_coordinate};
use crate::payment::{Payment, ResearchPayment};

mod map;
mod payment;
mod ui;

enum ActiveDialog {
    None,
    ResearchPayment(ResearchPayment),
}

struct State {
    focused_city: Option<(usize, Position)>,
    active_dialog: ActiveDialog,
}

#[macroquad::main("Clash")]
async fn main() {
    let mut game = Game::new(1, "a".repeat(32));
    let city = City::new(0, Position::from_offset("A1"));
    let player = &mut game.players[0];
    player.gain_resources(ResourcePile::new(50, 50, 50, 50, 50, 50, 50));
    player.cities.push(city);
    player
        .cities
        .push(City::new(0, Position::from_offset("C2")));
    player
        .cities
        .push(City::new(0, Position::from_offset("C1")));

    let mut state = State {
        active_dialog: ActiveDialog::None,
        focused_city: None,
    };

    loop {
        clear_background(GREEN);

        draw_map(&game);
        show_research_menu(&mut game, 0, &mut state);
        show_resources(&game, 0);

        if let Some((player_index, city_position)) = &state.focused_city {
            show_city_menu(&mut game, *player_index, city_position);
        }

        match &mut state.active_dialog {
            ActiveDialog::ResearchPayment(p) => {
                if buy_research_menu(&mut game, p) {
                    state.active_dialog = ActiveDialog::None;
                }
            }
            ActiveDialog::None => {}
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();

            let c = pixel_to_coordinate(x, y);

            state.focused_city = None;

            for p in game.players.iter() {
                for city in p.cities.iter() {
                    let pos = city.position.clone();
                    if c == pos.coordinate() {
                        state.focused_city = Some((p.index, pos));
                    };
                }
            }
        }

        next_frame().await
    }
}

fn draw_map(game: &Game) {
    for p in game.players.iter() {
        for city in p.cities.iter() {
            map::draw_city(p, city);
        }
    }
}

fn show_resources(game: &Game, player_index: usize) {
    let player = &game.players[player_index];
    let r: &ResourcePile = player.resources();

    let mut i: f32 = 0.;
    let mut res = |label: String| {
        draw_text(
            &label,
            600.,
            300. + player_index as f32 * 200. + i,
            20.,
            BLACK,
        );
        i += 30.;
    };

    res(format!("Food {}", r.food));
    res(format!("Wood {}", r.wood));
    res(format!("Ore {}", r.ore));
    res(format!("Ideas {}", r.ideas));
    res(format!("Gold {}", r.gold));
    res(format!("Mood {}", r.mood_tokens));
    res(format!("Culture {}", r.culture_tokens));
}

fn show_city_menu(game: &mut Game, player_index: usize, city_position: &Position) {
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

fn show_research_menu(game: &mut Game, player_index: usize, state: &mut State) {
    root_ui().window(hash!(), vec2(20., 300.), vec2(400., 200.), |ui| {
        for a in get_technologies().into_iter() {
            let name = a.name;
            if game.players[player_index].can_advance(&name) {
                if ui.button(None, name.clone()) {
                    state.active_dialog = ActiveDialog::ResearchPayment(ResearchPayment {
                        player_index,
                        name: name.clone(),
                        payment: Payment::new_advance_resource_payment(
                            game.players[player_index]
                                .resources()
                                .get_advance_payment_options(2),
                        ),
                    });
                }
            } else if game.players[player_index].advances.contains(&name) {
                ui.label(None, &name);
            }
        }
    });
}

fn buy_research_menu(game: &mut Game, rp: &mut ResearchPayment) -> bool {
    let mut result = false;
    root_ui().window(hash!(), vec2(20., 510.), vec2(400., 200.), |ui| {
        for (i, p) in rp.payment.resources.iter_mut().enumerate() {
            if p.max > 0 {
                Group::new(hash!("res", i), Vec2::new(70., 200.)).ui(ui, |ui| {
                    let s = format!("{} {}", &p.resource.to_string(), p.current);
                    ui.label(Vec2::new(0., 0.), &s);
                    if p.current > p.min && ui.button(Vec2::new(0., 20.), "-") {
                        p.current -= 1;
                    }
                    if p.current < p.max && ui.button(Vec2::new(20., 20.), "+") {
                        p.current += 1;
                    };
                });
            }
        }

        let label: &str = if rp.valid() { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) {
            if rp.valid() {
                game.execute_action(
                    Action::PlayingAction(PlayingAction::Advance {
                        advance: rp.name.clone(),
                        payment: rp.payment.to_resource_pile(),
                    }),
                    rp.player_index,
                );
                result = true;
            }
        };
    });
    return result;
}
