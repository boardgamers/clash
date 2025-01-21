use std::collections::HashMap;
use std::iter;

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{
    cancel_button, ok_button, BaseOrCustomAction, BaseOrCustomDialog, OkTooltip,
};
use crate::hex_ui;
use crate::hex_ui::Point;
use crate::layout_ui::{
    draw_icon, draw_scaled_icon, is_in_circle, left_mouse_button_pressed, ICON_SIZE,
};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name, show_resource_pile};
use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::WHITE;
use macroquad::shapes::draw_circle;
use server::action::Action;
use server::consts::PORT_CHOICES;
use server::content::custom_actions::CustomAction;
use server::game::Game;
use server::playing_actions::{get_total_collection, Collect, PlayingAction};
use server::position::Position;
use server::resource::{resource_types, ResourceType};
use server::resource_pile::ResourcePile;

#[derive(Clone)]
pub struct CollectResources {
    player_index: usize,
    city_position: Position,
    possible_collections: HashMap<Position, Vec<ResourcePile>>,
    collections: Vec<(Position, ResourcePile)>,
    custom: BaseOrCustomDialog,
}

impl CollectResources {
    pub fn new(
        player_index: usize,
        city_position: Position,
        possible_collections: HashMap<Position, Vec<ResourcePile>>,
        custom: BaseOrCustomDialog,
    ) -> CollectResources {
        CollectResources {
            player_index,
            city_position,
            collections: vec![],
            possible_collections,
            custom,
        }
    }

    fn get_collection(&self, p: Position) -> Option<&ResourcePile> {
        self.collections
            .iter()
            .find(|(pos, _)| pos == &p)
            .map(|(_, r)| r)
    }

    pub fn help_text(&self, game: &Game) -> Vec<String> {
        let extra = self.extra_resources(game);
        vec![
            self.custom.title.clone(),
            "Click on a tile to collect resources".to_string(),
            format!("{extra} left"),
        ]
    }

    pub fn extra_resources(&self, game: &Game) -> i8 {
        let city = game.get_city(self.player_index, self.city_position);
        city.mood_modified_size() as i8 - self.collections.len() as i8
    }

    pub fn collected(&self) -> ResourcePile {
        self.collections.clone().into_iter().map(|(_p, r)| r).sum()
    }
}

pub fn collect_dialog(rc: &RenderContext, collect: &CollectResources) -> StateUpdate {
    show_resource_pile(rc, &collect.collected(), &[]);

    let game = rc.game;
    let city = game.get_city(collect.player_index, collect.city_position);

    let tooltip = get_total_collection(
        game,
        collect.player_index,
        collect.city_position,
        &collect.collections,
    )
    .map_or(
        OkTooltip::Invalid("Too many resources selected".to_string()),
        |p| OkTooltip::Valid(format!("Collect {p}")),
    );
    if ok_button(rc, tooltip) {
        let extra = collect.extra_resources(game);

        let c = Collect {
            city_position: collect.city_position,
            collections: collect.collections.clone(),
        };
        let action = match collect.custom.custom {
            BaseOrCustomAction::Base => PlayingAction::Collect(c),
            BaseOrCustomAction::Custom { .. } => {
                PlayingAction::Custom(CustomAction::FreeEconomyCollect(c))
            }
        };
        return StateUpdate::execute_activation(
            Action::Playing(action),
            if extra > 0 {
                vec![format!("{extra} more tiles can be collected")]
            } else {
                vec![]
            },
            city,
        );
    };
    if cancel_button(rc) {
        return StateUpdate::Cancel;
    };
    StateUpdate::None
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
            if city
                .port_position
                .is_some_and(|p| p == pos && !is_blocked(game, player_index, p))
            {
                return Some((pos, PORT_CHOICES.to_vec()));
            }
            if let Some(t) = game.map.get(pos) {
                if let Some(option) = collect_options
                    .get(t)
                    .filter(|_| pos == city_pos || !is_blocked(game, player_index, pos))
                {
                    return Some((pos, option.clone()));
                }
            }
            None
        })
        .collect()
}

fn click_collect_option(col: &CollectResources, p: Position, pile: &ResourcePile) -> StateUpdate {
    let mut new = col.clone();
    let old = col.collections.iter().find(|(pos, _)| pos == &p);

    new.collections.retain(|(pos, _)| pos != &p);
    if old.is_none_or(|(_, r)| r != pile) {
        new.collections.push((p, pile.clone()));
    }

    StateUpdate::OpenDialog(ActiveDialog::CollectResources(new))
}

pub fn draw_resource_collect_tile(rc: &RenderContext, pos: Position) -> StateUpdate {
    let state = &rc.state;
    if let ActiveDialog::CollectResources(collect) = &state.active_dialog {
        if let Some(possible) = collect.possible_collections.get(&pos) {
            let col = collect.get_collection(pos);

            let c = hex_ui::center(pos);
            for (i, pile) in possible.iter().enumerate() {
                let center = if possible.len() == 1 {
                    c
                } else {
                    hex_ui::rotate_around(c, 30.0, 90 * i)
                };
                let color = if col.is_some_and(|r| r == pile) {
                    BLACK
                } else {
                    WHITE
                };
                draw_circle(center.x, center.y, 20., color);
                if let Some(p) = left_mouse_button_pressed(rc) {
                    if is_in_circle(p, center, 20.) {
                        return click_collect_option(collect, pos, pile);
                    }
                }

                let map = new_resource_map(pile);
                let m: Vec<(ResourceType, &u32)> = resource_types()
                    .iter()
                    .filter_map(|r| {
                        let a = map.get(r);
                        a.is_some_and(|a| *a > 0).then(|| (*r, a.unwrap()))
                    })
                    .collect();
                draw_collect_item(rc, center, &m);
            }
        }
    };
    StateUpdate::None
}

fn draw_collect_item(rc: &RenderContext, center: Point, resources: &[(ResourceType, &u32)]) {
    if resources.iter().len() == 1 {
        let (r, _) = resources.first().unwrap();
        draw_icon(
            rc,
            &rc.assets().resources[r],
            resource_name(*r),
            center.to_vec2() - vec2(ICON_SIZE / 2., ICON_SIZE / 2.),
        );
    } else {
        resources.iter().enumerate().for_each(|(j, (r, _))| {
            let size = ICON_SIZE / 2.;
            let c = hex_ui::rotate_around(center, 10.0, 180 * j);
            draw_scaled_icon(
                rc,
                &rc.assets().resources[r],
                resource_name(*r),
                c.to_vec2() - vec2(size / 2., size / 2.),
                size,
            );
        });
    }
}

fn is_blocked(game: &Game, player_index: usize, pos: Position) -> bool {
    game.get_any_city(pos).is_some() || game.enemy_player(player_index, pos).is_some()
}
