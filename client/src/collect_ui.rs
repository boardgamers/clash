use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{
    cancel_button, ok_button, BaseOrCustomAction, BaseOrCustomDialog, OkTooltip,
};
use crate::event_ui::event_help;
use crate::hex_ui;
use crate::hex_ui::Point;
use crate::layout_ui::{draw_scaled_icon, is_in_circle, left_mouse_button_pressed};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name, show_resource_pile};
use macroquad::color::BLACK;
use macroquad::math::vec2;
use macroquad::prelude::WHITE;
use macroquad::shapes::draw_circle;
use server::action::Action;
use server::collect::{get_total_collection, possible_resource_collections, CollectOptionsInfo};
use server::content::custom_actions::CustomAction;
use server::game::Game;
use server::playing_actions::{Collect, PlayingAction};
use server::position::Position;
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;

#[derive(Clone)]
pub struct CollectResources {
    player_index: usize,
    city_position: Position,
    collections: Vec<(Position, ResourcePile)>,
    custom: BaseOrCustomDialog,
    pub info: CollectOptionsInfo,
}

impl CollectResources {
    pub fn new(
        player_index: usize,
        city_position: Position,
        custom: BaseOrCustomDialog,
        info: CollectOptionsInfo,
    ) -> CollectResources {
        CollectResources {
            player_index,
            city_position,
            collections: vec![],
            custom,
            info,
        }
    }

    fn get_collection(&self, p: Position) -> Option<&ResourcePile> {
        self.collections
            .iter()
            .find(|(pos, _)| pos == &p)
            .map(|(_, r)| r)
    }

    pub fn help_text(&self, rc: &RenderContext) -> Vec<String> {
        let extra = self.extra_resources(rc.game);
        let mut r = vec![
            self.custom.title.clone(),
            "Click on a tile to collect resources".to_string(),
            format!("{extra} left"),
        ];
        for o in self.info.modifiers.clone() {
            let vec1 = event_help(rc, &o, true);
            r.extend(vec1);
        }
        r
    }

    pub fn extra_resources(&self, game: &Game) -> i8 {
        let city = game.get_city(self.player_index, self.city_position);
        city.mood_modified_size(game.get_player(self.player_index)) as i8
            - self.collections.len() as i8
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
        |(_, p)| OkTooltip::Valid(format!("Collect {p}")),
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

fn click_collect_option(
    rc: &RenderContext,
    col: &CollectResources,
    p: Position,
    pile: &ResourcePile,
) -> StateUpdate {
    let mut new = col.clone();
    let old = col.collections.iter().find(|(pos, _)| pos == &p);

    new.collections.retain(|(pos, _)| pos != &p);
    if old.is_none_or(|(_, r)| r != pile) {
        new.collections.push((p, pile.clone()));
    }

    let used = new.collections.clone().into_iter().collect();
    let i = possible_resource_collections(rc.game, col.city_position, col.player_index, &used, &[]);
    new.info = i;
    StateUpdate::OpenDialog(ActiveDialog::CollectResources(new))
}

pub fn draw_resource_collect_tile(rc: &RenderContext, pos: Position) -> StateUpdate {
    let state = &rc.state;
    if let ActiveDialog::CollectResources(collect) = &state.active_dialog {
        if let Some(possible) = collect.info.choices.get(&pos) {
            let col = collect.get_collection(pos);

            let c = hex_ui::center(pos);
            for (i, pile) in possible.iter().enumerate() {
                let deg = (360. / possible.len() as f32) as usize * i;
                let (center, radius) = match possible.len() {
                    1 => (c, 20.),
                    2 => (hex_ui::rotate_around(c, 30.0, deg), 20.),
                    n if n <= 4 => (hex_ui::rotate_around(c, 30.0, deg), 20.),
                    _ => (hex_ui::rotate_around(c, 30.0, deg), 10.),
                };
                let size = radius * 1.3;

                let color = if col.is_some_and(|r| r == pile) {
                    BLACK
                } else {
                    WHITE
                };
                draw_circle(center.x, center.y, radius, color);
                if let Some(p) = left_mouse_button_pressed(rc) {
                    if is_in_circle(p, center, radius) {
                        return click_collect_option(rc, collect, pos, pile);
                    }
                }

                let map = new_resource_map(pile);
                let m: Vec<(ResourceType, &u32)> = ResourceType::all()
                    .iter()
                    .filter_map(|r| {
                        let a = map.get(r);
                        a.is_some_and(|a| *a > 0).then(|| (*r, a.unwrap()))
                    })
                    .collect();
                draw_collect_item(rc, center, &m, size);
            }
        }
    };
    StateUpdate::None
}

fn draw_collect_item(
    rc: &RenderContext,
    center: Point,
    resources: &[(ResourceType, &u32)],
    size: f32,
) {
    if resources.iter().len() == 1 {
        let (r, _) = resources.first().unwrap();
        draw_scaled_icon(
            rc,
            &rc.assets().resources[r],
            resource_name(*r),
            center.to_vec2() - vec2(size / 2., size / 2.),
            size,
        );
    } else {
        resources.iter().enumerate().for_each(|(j, (r, _))| {
            let c = hex_ui::rotate_around(center, 10.0, 180 * j);
            draw_scaled_icon(
                rc,
                &rc.assets().resources[r],
                resource_name(*r),
                c.to_vec2() - vec2(size / 2., size / 2.),
                size / 2.,
            );
        });
    }
}
