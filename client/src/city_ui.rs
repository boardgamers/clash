use macroquad::hash;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;

use crate::collect_ui::CollectResources;
use crate::construct_ui::{add_construct_button, add_wonder_buttons};
use crate::happiness_ui::increase_happiness_click;
use crate::hex_ui::{draw_hex_center_text, pixel_to_coordinate};
use crate::ui_state::{can_play_action, CityMenu, State};
use crate::{hex_ui, influence_ui, player_ui, ActiveDialog};

pub fn show_city_menu(game: &mut Game, menu: CityMenu) -> Option<ActiveDialog> {
    let mut result: Option<ActiveDialog> = None;

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        ui.label(None, &menu.city_position.to_string());

        if ui.button(None, "Collect Resources") {
            result = Some(ActiveDialog::CollectResources(CollectResources::new(
                menu.player_index,
                menu.city_position.clone(),
            )));
        }

        add_building_actions(game, &menu, &mut result, ui);

        if can_play_action(game) && menu.is_city_owner() {
            if let Some(d) = add_wonder_buttons(game, &menu, ui) {
                let _ = result.insert(d);
            }
        }
    });
    result
}

fn add_building_actions(
    game: &mut Game,
    menu: &CityMenu,
    result: &mut Option<ActiveDialog>,
    ui: &mut Ui,
) {
    let closest_city_pos = &influence_ui::closest_city(&game, menu);

    for (building, name) in building_names() {
        if can_play_action(game) {
            if let Some(d) = add_construct_button(game, menu, ui, &building, name) {
                let _ = result.insert(d);
            }

            influence_ui::add_influence_button(game, menu, ui, closest_city_pos, &building, name);
        };
    }
}

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    let c = hex_ui::center(&city.position).to_screen();

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, player_ui::player_color(owner.index));

    if let Some(increase) = &state.increase_happiness {
        let steps = increase
            .steps
            .iter()
            .find(|(p, _)| p == &city.position)
            .map_or(String::new(), |(_, s)| format!("{}", s));
        draw_hex_center_text(&city.position, &steps);
    } else {
        match city.mood_state {
            MoodState::Happy => draw_hex_center_text(&city.position, "+"),
            MoodState::Neutral => {}
            MoodState::Angry => draw_hex_center_text(&city.position, "-"),
        }
    }

    let mut i = 0;
    city.city_pieces.wonders.iter().for_each(|w| {
        let p = hex_ui::rotate_around(c, 30.0, 90 * i);
        draw_text(
            &w.name,
            p.x - 12.0,
            p.y + 12.0,
            50.0,
            player_ui::player_color(owner.index),
        );
        i += 1;
    });

    for player_index in 0..4 {
        for b in city.city_pieces.buildings(Some(player_index)).iter() {
            let p = hex_ui::rotate_around(c, 30.0, 90 * i);
            draw_text(
                building_symbol(b),
                p.x - 12.0,
                p.y + 12.0,
                50.0,
                player_ui::player_color(player_index),
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

fn building_names() -> [(Building, &'static str); 7] {
    [
        (Building::Academy, "Academy"),
        (Building::Market, "Market"),
        (Building::Obelisk, "Obelisk"),
        (Building::Observatory, "Observatory"),
        (Building::Fortress, "Fortress"),
        (Building::Port, "Port"),
        (Building::Temple, "Temple"),
    ]
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
