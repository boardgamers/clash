use std::collections::HashMap;
use std::iter;

use macroquad::color::RED;
use macroquad::math::i32;
use macroquad::prelude::{draw_circle_lines, draw_text, Vec2, WHITE};
use server::action::Action;
use server::consts::PORT_CHOICES;
use server::game::Game;
use server::playing_actions::{get_total_collection, PlayingAction};
use server::position::Position;
use server::resource_pile::ResourcePile;

use crate::dialog_ui::active_dialog_window;
use crate::hex_ui;
use crate::resource_ui::resource_pile_string;
use crate::ui_state::{ActiveDialog, State, StateUpdate};

#[derive(Clone)]
pub struct CollectResources {
    player_index: usize,
    city_position: Position,
    possible_collections: HashMap<Position, Vec<ResourcePile>>,
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

    fn get_collection(&self, p: Position) -> Option<&ResourcePile> {
        self.collections
            .iter()
            .find(|(pos, _)| pos == &p)
            .map(|(_, r)| r)
    }
}

pub fn collect_resources_dialog(game: &Game, collect: &CollectResources) -> StateUpdate {
    active_dialog_window(|ui, updates| {
        let city = game.get_city(collect.player_index, collect.city_position);
        let valid = get_total_collection(
            game,
            collect.player_index,
            collect.city_position,
            &collect.collections,
        )
        .is_some();
        let label = if valid { "OK" } else { "(OK)" };
        if ui.button(Vec2::new(0., 40.), label) && valid {
            let extra = city.mood_modified_size() - collect.collections.len();

            updates.add(StateUpdate::execute_activation(
                Action::Playing(PlayingAction::Collect {
                    city_position: collect.city_position,
                    collections: collect.collections.clone(),
                }),
                if extra > 0 {
                    vec![format!("{extra} more tiles can be collected")]
                } else {
                    vec![]
                },
                city,
            ));
        };
        if ui.button(Vec2::new(80., 40.), "Cancel") {
            updates.add(StateUpdate::Cancel);
        };
    })
}

pub fn possible_resource_collections(
    game: &Game,
    city_pos: Position,
    player_index: usize,
) -> HashMap<Position, Vec<ResourcePile>> {
    let collect_options = &game.get_player(player_index).collect_options;
    let city = game.get_city(player_index, city_pos);
    city_pos
        .neighbors()
        .into_iter()
        .chain(iter::once(city_pos))
        .filter_map(|pos| {
            if city.port_position.is_some_and(|p| p == pos) {
                return Some((pos, PORT_CHOICES.to_vec()));
            }
            if let Some(t) = game.map.tiles.get(&pos) {
                if let Some(option) = collect_options
                    .get(t)
                    .filter(|_| pos == city_pos || !is_blocked(game, pos))
                {
                    return Some((pos, option.clone()));
                }
            }
            None
        })
        .collect()
}

pub fn click_collect_option(col: &CollectResources, p: Position) -> StateUpdate {
    let mut new = col.clone();
    if let Some(possible) = new.possible_collections.get(&p) {
        if let Some(current) = new
            .get_collection(p)
            .and_then(|r| possible.iter().position(|p| p == r))
        {
            new.collections.retain(|(pos, _)| pos != &p);
            let next = current + 1;
            if next < possible.len() {
                new.collections.push((p, possible[next].clone()));
            }
        } else {
            new.collections.push((p, possible[0].clone()));
        }
        return StateUpdate::SetDialog(ActiveDialog::CollectResources(new));
    }
    StateUpdate::None
}

pub fn draw_resource_collect_tile(state: &State, pos: Position) {
    if let ActiveDialog::CollectResources(collect) = &state.active_dialog {
        if let Some(possible) = collect.possible_collections.get(&pos) {
            draw_circle_lines(
                hex_ui::center(pos).x,
                hex_ui::center(pos).y,
                18.0,
                2.0,
                WHITE,
            );

            let col = collect.get_collection(pos);

            let c = hex_ui::center(pos);
            possible.iter().enumerate().for_each(|(i, res)| {
                let p = hex_ui::rotate_around(c, 30.0, (90 * i) as i32);
                let color = if col.is_some_and(|r| r == res) {
                    WHITE
                } else {
                    RED
                };
                draw_text(
                    &resource_pile_string(res),
                    p.x - 12.0,
                    p.y + 12.0,
                    50.0,
                    color,
                );
            });
        }
    };
}

fn is_blocked(game: &Game, pos: Position) -> bool {
    //todo also look for enemy units
    if game.get_any_city(pos).is_some() {
        return true;
    }
    false
}
