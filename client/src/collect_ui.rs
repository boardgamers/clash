use std::collections::HashMap;
use std::iter;

use macroquad::color::RED;
use macroquad::math::i32;
use macroquad::prelude::{draw_circle_lines, draw_text, Vec2, WHITE};
use server::action::Action;
use server::game::Game;
use server::map::Terrain;
use server::playing_actions::{get_total_collection, PlayingAction, PORT_CHOICES};
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::dialog_ui::active_dialog_window;
use crate::hex_ui;
use crate::resource_ui::resource_symbol;
use crate::ui_state::{ActiveDialog, State};

pub struct CollectResources {
    pub player_index: usize,
    pub city_position: Position,
    pub possible_collections: HashMap<Position, Vec<ResourcePile>>,
    collections: Vec<(Position, ResourcePile)>,
}

impl CollectResources {
    pub fn new(
        player_index: usize,
        city_position: Position,
        possible_collections: HashMap<Position, Vec<ResourcePile>>,
    ) -> CollectResources {
        CollectResources {
            player_index,
            city_position,
            collections: vec![],
            possible_collections,
        }
    }

    fn get_collection(&self, p: &Position) -> Option<&ResourcePile> {
        self.collections
            .iter()
            .find(|(pos, _)| pos == p)
            .map(|(_, r)| r)
    }
}

pub fn collect_resources_dialog(game: &mut Game, collect: &CollectResources) -> bool {
    let mut result = false;
    active_dialog_window(|ui| {
        let _city = game.get_city(collect.player_index, &collect.city_position);
        let valid = get_total_collection(
            game,
            collect.player_index,
            &collect.city_position,
            &collect.collections,
        )
        .is_some();
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) && valid {
            game.execute_action(
                Action::Playing(PlayingAction::Collect {
                    city_position: collect.city_position.clone(),
                    collections: collect.collections.clone(),
                }),
                collect.player_index,
            );

            result = true;
        };
        if ui.button(Vec2::new(80., 40.), "Cancel") {
            result = true;
        };
    });
    result
}

pub fn possible_resource_collections(
    game: &Game,
    city_pos: &Position,
    player_index: usize,
) -> HashMap<Position, Vec<ResourcePile>> {
    let collect_options = &game.get_player(player_index).collect_options;
    let city = game.get_city(player_index, city_pos);
    city_pos
        .neighbors()
        .iter()
        .chain(iter::once(city_pos))
        .flat_map(|pos| {
            if city.port_position.as_ref().is_some_and(|p| p == pos) {
                return Some((pos.clone(), PORT_CHOICES.to_vec()));
            }
            if let Some(t) = game.map.tiles.get(pos) {
                if let Some(option) = collect_options
                    .get(t)
                    .filter(|_| pos == city_pos || !is_blocked(game, pos))
                {
                    return Some((pos.clone(), option.clone()));
                }
            }
            None
        })
        .collect()
}

pub fn click_collect_option(col: &mut CollectResources, p: &Position) {
    if let Some(possible) = col.possible_collections.get(p) {
        if let Some(current) = col
            .get_collection(p)
            .and_then(|r| possible.iter().position(|p| p == r))
        {
            col.collections.retain(|(pos, _)| pos != p);
            let next = current + 1;
            if next < possible.len() {
                col.collections.push((p.clone(), possible[next].clone()));
            }
        } else {
            col.collections.push((p.clone(), possible[0].clone()));
        }
    }
}

pub fn draw_resource_collect_tile(_game: &Game, state: &State, pos: &Position, _t: &Terrain) {
    if let ActiveDialog::CollectResources(collect) = &state.active_dialog {
        if let Some(possible) = collect.possible_collections.get(pos) {
            draw_circle_lines(
                hex_ui::center(pos).to_screen().x,
                hex_ui::center(pos).to_screen().y,
                18.0,
                2.0,
                WHITE,
            );

            let col = collect.get_collection(pos);

            let c = hex_ui::center(pos).to_screen();
            possible.iter().enumerate().for_each(|(i, res)| {
                let p = hex_ui::rotate_around(c, 30.0, (90 * i) as i32);
                let color = if col.is_some_and(|r| r == res) {
                    WHITE
                } else {
                    RED
                };
                draw_text(&resource_symbol(res), p.x - 12.0, p.y + 12.0, 50.0, color);
            });
        }
    };
}

fn is_blocked(game: &Game, pos: &Position) -> bool {
    //todo also look for enemy units
    for p in game.players.iter() {
        for city in p.cities.iter() {
            if city.position == *pos {
                return true;
            }
        }
    }
    false
}
