use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::{BaseOrCustomDialog, OkTooltip, cancel_button, ok_button};
use crate::event_ui::event_help;
use crate::hex_ui;
use crate::layout_ui::{draw_scaled_icon, is_in_circle, mouse_pressed_position};
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name, show_resource_pile};
use itertools::Itertools;
use macroquad::color::BLACK;
use macroquad::math::{Vec2, vec2};
use macroquad::prelude::WHITE;
use macroquad::shapes::draw_circle;
use server::action::Action;
use server::collect::{
    Collect, CollectInfo, PositionCollection, add_collect, get_total_collection,
    possible_resource_collections, tiles_used,
};
use server::events::check_event_origin;
use server::player::CostTrigger;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource::ResourceType;
use server::resource_pile::ResourcePile;

#[derive(Clone, Debug)]
pub struct CollectResources {
    player_index: usize,
    city_position: Position,
    collections: Vec<PositionCollection>,
    custom: BaseOrCustomDialog,
    pub info: CollectInfo,
}

impl CollectResources {
    pub fn new(
        player_index: usize,
        city_position: Position,
        custom: BaseOrCustomDialog,
        info: CollectInfo,
    ) -> CollectResources {
        CollectResources {
            player_index,
            city_position,
            collections: vec![],
            custom,
            info,
        }
    }

    pub fn help_text(&self, rc: &RenderContext) -> Vec<String> {
        let extra = self.extra_resources();
        let mut r = vec![
            self.custom.title.clone(),
            "Click on a tile to collect resources".to_string(),
            format!("{extra} left"),
        ];
        for o in self.info.modifiers.clone() {
            r.extend(event_help(rc, &o));
        }
        r
    }

    pub fn extra_resources(&self) -> i8 {
        self.info.max_selection as i8 - tiles_used(&self.collections) as i8
    }

    pub fn collected(&self) -> ResourcePile {
        self.collections
            .clone()
            .into_iter()
            .map(|c| c.total())
            .sum()
    }
}

pub fn collect_dialog(rc: &RenderContext, collect: &CollectResources) -> RenderResult {
    show_resource_pile(rc, &collect.collected());

    let game = rc.game;
    let city = game.city(collect.player_index, collect.city_position);

    let result = get_total_collection(
        game,
        collect.player_index,
        &check_event_origin(),
        collect.city_position,
        &collect.collections,
        CostTrigger::WithModifiers,
    );
    let tooltip = result.as_ref().map_or(
        OkTooltip::Invalid("Too many resources selected".to_string()),
        |i| OkTooltip::Valid(format!("Collect {}", i.total)),
    );
    if ok_button(rc, tooltip) {
        let extra = collect.extra_resources();

        let c = Collect::new(
            collect.city_position,
            collect.collections.clone(),
            collect.custom.action_type.clone(),
        );

        return StateUpdate::execute_activation(
            Action::Playing(PlayingAction::Collect(c)),
            if extra > 0 {
                vec![format!("{extra} more tiles can be collected")]
            } else {
                vec![]
            },
            city,
        );
    }
    if cancel_button(rc) {
        return StateUpdate::cancel();
    }
    NO_UPDATE
}

fn click_collect_option(
    rc: &RenderContext,
    col: &CollectResources,
    p: Position,
    pile: &ResourcePile,
) -> RenderResult {
    let c = add_collect(&col.info, p, pile, &col.collections);

    let i = possible_resource_collections(
        rc.game,
        col.info.city,
        col.player_index,
        &check_event_origin(),
        CostTrigger::WithModifiers,
    );
    let mut new = col.clone();
    new.info = i;
    new.collections = c;
    StateUpdate::open_dialog(ActiveDialog::CollectResources(new))
}

pub fn draw_resource_collect_tile(rc: &RenderContext, pos: Position) -> RenderResult {
    let state = &rc.state;
    let ActiveDialog::CollectResources(collect) = &state.active_dialog else {
        return NO_UPDATE;
    };

    let Some(possible) = collect
        .info
        .choices
        .get(&pos)
        .map(|v| v.iter().sorted_by_key(ToString::to_string).collect_vec())
    else {
        return NO_UPDATE;
    };

    let tile_collects = collect
        .collections
        .iter()
        .filter(|c| c.position == pos)
        .collect_vec();

    let c = hex_ui::center(pos);
    for (i, &pile) in possible.iter().enumerate() {
        let deg = (360. / possible.len() as f32) as usize * i;
        let (center, radius) = match possible.len() {
            1 => (c, 20.),
            2 => (hex_ui::rotate_around(c, 30.0, deg), 20.),
            n if n <= 4 => (hex_ui::rotate_around(c, 30.0, deg), 20.),
            _ => (hex_ui::rotate_around(c, 30.0, deg), 10.),
        };
        let size = radius * 1.3;

        let pile_collect = tile_collects.iter().find(|r| &r.pile == pile);
        let color = if pile_collect.is_some() { BLACK } else { WHITE };
        draw_circle(center.x, center.y, radius, color);
        if let Some(p) = mouse_pressed_position(rc) {
            if is_in_circle(p, center, radius) {
                return click_collect_option(rc, collect, pos, pile);
            }
        }

        let map = new_resource_map(pile);
        let m: Vec<(ResourceType, &u8)> = ResourceType::all()
            .iter()
            .filter_map(|r| {
                let a = map.get(r);
                a.is_some_and(|a| *a > 0).then(|| (*r, a.unwrap()))
            })
            .collect();
        draw_collect_item(rc, center, &m, size);

        if let Some(r) = pile_collect {
            let times = r.times;
            if times > 1 {
                rc.state
                    .draw_text(&format!("{times}"), center.x + 20., center.y + 5.);
            }
        }
    }
    NO_UPDATE
}

fn draw_collect_item(
    rc: &RenderContext,
    center: Vec2,
    resources: &[(ResourceType, &u8)],
    size: f32,
) {
    if resources.iter().len() == 1 {
        let (r, _) = resources.first().unwrap();
        draw_scaled_icon(
            rc,
            &rc.assets().resources[r],
            resource_name(*r),
            center - vec2(size / 2., size / 2.),
            size,
        );
    } else {
        resources.iter().enumerate().for_each(|(j, (r, _))| {
            let c = hex_ui::rotate_around(center, 10.0, 180 * j);
            draw_scaled_icon(
                rc,
                &rc.assets().resources[r],
                resource_name(*r),
                c - vec2(size / 2., size / 2.),
                size / 2.,
            );
        });
    }
}
