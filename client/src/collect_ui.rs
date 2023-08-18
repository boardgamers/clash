use macroquad::color::RED;
use macroquad::math::i32;
use macroquad::prelude::{draw_circle_lines, draw_text, Vec2, WHITE};
use server::action::Action;
use server::game::Game;
use server::map::Terrain;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::dialog_ui::active_dialog_window;
use crate::hex_ui;
use crate::resource_ui::resource_symbol;
use crate::ui_state::{ActiveDialog, State};

pub struct CollectResources {
    pub player_index: usize,
    pub city_position: Position,
    pub current_tile: Option<Position>,
    collections: Vec<(Position, ResourcePile)>,
}

impl CollectResources {
    pub fn new(player_index: usize, city_position: Position) -> CollectResources {
        CollectResources {
            player_index,
            city_position,
            current_tile: None,
            collections: vec![],
        }
    }
}

pub fn collect_resources_dialog(game: &mut Game, collect: &CollectResources) -> bool {
    let mut result = false;
    active_dialog_window(|ui| {
        let valid = false;
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

pub fn draw_resource_collect_tile(game: &Game, state: &State, pos: &Position, t: &Terrain) {
    if let ActiveDialog::CollectResources(collect) = &state.active_dialog {
        let city = &collect.city_position;
        if pos.is_neighbor(city) || pos == city {
            if let Some(option) = game
                .get_player(game.current_player_index)
                .collect_options
                .get(t)
                .filter(|_| pos == city || !is_blocked(game, pos)) {
                draw_circle_lines(
                    hex_ui::center(pos).to_screen().x,
                    hex_ui::center(pos).to_screen().y,
                    18.0,
                    2.0,
                    WHITE,
                );

                let c = hex_ui::center(pos).to_screen();
                option.iter().enumerate().for_each(|(i, r)| {
                    let p = hex_ui::rotate_around(c, 30.0, (90 * i) as i32);
                    draw_text(&resource_symbol(r), p.x - 12.0, p.y + 12.0, 50.0, RED);
                });
            }
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
