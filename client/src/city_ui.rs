use macroquad::hash;
use macroquad::math::vec2;
use macroquad::prelude::*;
use macroquad::ui::{root_ui, Ui};
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::unit::Units;

use crate::collect_ui::{possible_resource_collections, CollectResources};
use crate::construct_ui::{add_construct_button, add_wonder_buttons};
use crate::happiness_ui::init_increase_happiness;
use crate::hex_ui::draw_hex_center_text;
use crate::recruit_unit_ui::RecruitAmount;
use crate::ui_state::{can_play_action, CityMenu, State, StateUpdate, StateUpdates};
use crate::{hex_ui, influence_ui, player_ui, ActiveDialog};

pub fn show_city_menu(game: &Game, menu: &CityMenu) -> StateUpdate {
    let mut updates: StateUpdates = StateUpdates::new();

    root_ui().window(hash!(), vec2(30., 700.), vec2(500., 200.), |ui| {
        ui.label(None, &menu.city_position.to_string());

        let can_play =
            can_play_action(game) && menu.is_city_owner() && menu.get_city(game).can_activate();
        if can_play {
            if ui.button(None, "Collect Resources") {
                updates.add(StateUpdate::SetDialog(ActiveDialog::CollectResources(
                    CollectResources::new(
                        menu.player_index,
                        menu.city_position,
                        possible_resource_collections(
                            game,
                            menu.city_position,
                            menu.city_owner_index,
                        ),
                    ),
                )));
            }
            if ui.button(None, "Recruit Units") {
                updates.add(RecruitAmount::new_selection(
                    game,
                    menu.player_index,
                    menu.city_position,
                    Units::empty(),
                    &[],
                ));
            }
        }

        updates.add(add_building_actions(game, menu, ui));

        if can_play {
            let option = add_wonder_buttons(game, menu, ui);
            updates.add(option);
        }
    });
    updates.result()
}

fn add_building_actions(game: &Game, menu: &CityMenu, ui: &mut Ui) -> StateUpdate {
    let closest_city_pos = influence_ui::closest_city(game, menu);

    if !can_play_action(game) {
        return StateUpdate::None;
    }

    let mut updates = StateUpdates::new();
    for (building, name) in building_names() {
        updates.add(add_construct_button(game, menu, ui, &building, name));
        updates.add(influence_ui::add_influence_button(
            game,
            menu,
            ui,
            closest_city_pos,
            &building,
            name,
        ));
    }
    updates.result()
}

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    let c = hex_ui::center(city.position);

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, player_ui::player_color(owner.index));

    if let Some(increase) = &state.increase_happiness {
        let steps = increase
            .steps
            .iter()
            .find(|(p, _)| p == &city.position)
            .map_or(String::new(), |(_, s)| format!("{s}"));
        draw_hex_center_text(city.position, &steps);
    } else {
        match city.mood_state {
            MoodState::Happy => draw_hex_center_text(city.position, "+"),
            MoodState::Neutral => {}
            MoodState::Angry => draw_hex_center_text(city.position, "-"),
        }
    }

    let mut i = 0;
    city.pieces.wonders.iter().for_each(|w| {
        let p = hex_ui::rotate_around(c, 30.0, 90 * i);
        draw_text(
            &w.name,
            p.x - 10.0,
            p.y + 10.0,
            40.0,
            player_ui::player_color(owner.index),
        );
        i += 1;
    });

    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let p = hex_ui::rotate_around(c, 30.0, 90 * i);
            draw_text(
                building_symbol(b),
                p.x - 10.0,
                p.y + 10.0,
                40.0,
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

pub fn city_click(state: &State, player: &Player, city: &City) -> StateUpdate {
    let pos = city.position;

    if let Some(increase_happiness) = &state.increase_happiness {
        StateUpdate::SetIncreaseHappiness(init_increase_happiness(
            player,
            city,
            pos,
            increase_happiness,
        ))
    } else {
        StateUpdate::FocusCity(player.index, pos)
    }
}
