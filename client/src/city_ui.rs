use macroquad::prelude::*;
use macroquad::ui::Ui;

use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::status_phase::StatusPhaseAction;
use server::unit::Units;

use crate::collect_ui::{possible_resource_collections, CollectResources};
use crate::construct_ui::{add_construct_button, add_wonder_buttons};
use crate::dialog_ui::active_dialog_window;
use crate::happiness_ui::init_increase_happiness;
use crate::hex_ui::draw_hex_center_text;
use crate::map_ui::show_tile_menu;
use crate::recruit_unit_ui::RecruitAmount;
use crate::ui_state::{can_play_action, CityMenu, FocusedTile, State, StateUpdate, StateUpdates};
use crate::{hex_ui, influence_ui, player_ui, ActiveDialog};

pub fn show_city_menu(game: &Game, menu: &CityMenu) -> StateUpdate {
    let position = menu.city_position;
    let city = menu.get_city(game);
    let suffix = format!(
        "size: {} mood: {} activated: {}",
        city.size(),
        match city.mood_state {
            MoodState::Happy => "Happy",
            MoodState::Neutral => "Neutral",
            MoodState::Angry => "Angry",
        },
        city.is_activated(),
    );

    show_tile_menu(game, position, Some(&suffix), |ui, updates| {
        let can_play = can_play_action(game) && menu.is_city_owner() && city.can_activate();
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
                    None,
                    &[],
                ));
            }
        }

        updates.add(add_building_actions(game, menu, ui));

        if can_play {
            let option = add_wonder_buttons(game, menu, ui);
            updates.add(option);
        }
    })
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
        if player.index == city.player_index {
            StateUpdate::SetIncreaseHappiness(init_increase_happiness(
                player,
                city,
                pos,
                increase_happiness,
            ))
        } else {
            StateUpdate::None
        }
    } else {
        StateUpdate::FocusTile(FocusedTile::new(Some(city.player_index), pos))
    }
}

pub fn raze_city_dialog() -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        ui.label(None, "Select a city to raze - or decline.");
        if ui.button(None, "Decline") {
            updates.add(StateUpdate::status_phase(StatusPhaseAction::RaseSize1City(
                None,
            )));
        }
    });
    updates.result()
}
