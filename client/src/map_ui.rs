use crate::city_ui::{IconAction, IconActionVec, draw_city, show_city_menu};
use crate::client_state::{ActiveDialog, MAX_OFFSET, MIN_OFFSET, State, StateUpdate, ZOOM};
use crate::dialog_ui::{OkTooltip, cancel_button_pos, ok_button};
use crate::layout_ui::{
    bottom_center_anchor, bottom_center_texture, bottom_right_texture, icon_pos,
};
use crate::move_ui::{MoveDestination, MoveIntent, movable_units};
use crate::player_ui::get_combat;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::tooltip::show_tooltip_for_circle;
use crate::{collect_ui, hex_ui, unit_ui};
use macroquad::math::{f32, vec2};
use macroquad::prelude::*;
use server::action::Action;
    use server::content::persistent_events::EventResponse;
use server::map::{Rotation, Terrain, UnexploredBlock};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::unit::UnitType;
use std::collections::HashMap;
use std::ops::{Add, Mul, Rem, Sub};
use std::vec;
use server::combat_stats::CombatStats;

const MOVE_DESTINATION: Color = color(51, 255, 72, 0.4);

const fn color(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., a)
}

#[derive(Clone)]
pub struct ExploreResolutionConfig {
    pub block: UnexploredBlock,
    pub rotation: Rotation,
}

fn terrain_font_color(t: &Terrain) -> Color {
    match t {
        Terrain::Forest | Terrain::Water | Terrain::Fertile => WHITE,
        _ => BLACK,
    }
}

pub fn terrain_name(t: &Terrain) -> &'static str {
    match t {
        Terrain::Barren => "Barren",
        Terrain::Mountain => "Mountain",
        Terrain::Fertile => "Grassland",
        Terrain::Forest => "Forest",
        Terrain::Exhausted(_) => "Exhausted",
        Terrain::Water => "Water",
        Terrain::Unexplored => "Unexplored",
    }
}

pub fn draw_map(rc: &RenderContext) -> StateUpdate {
    let game = rc.game;
    let overlay_terrain = get_overlay(rc);
    for (pos, t) in &game.map.tiles {
        let terrain = overlay_terrain.get(pos).unwrap_or(t);
        let (base, exhausted) = match terrain {
            Terrain::Exhausted(e) => (e.as_ref(), true),
            _ => (terrain, false),
        };

        hex_ui::draw_hex(
            *pos,
            terrain_font_color(terrain),
            overlay_color(rc, *pos),
            rc.assets().terrain.get(base),
            exhausted,
            rc,
        );
        let update = collect_ui::draw_resource_collect_tile(rc, *pos);
        if !matches!(update, StateUpdate::None) {
            return update;
        }
    }
    // get from current event
    if let Some(c) = get_combat(game) {
        draw_combat_arrow(c);
    }
    let state = &rc.state;
    if !matches!(&state.active_dialog, ActiveDialog::CollectResources(_)) {
        for p in &game.players {
            for city in &p.cities {
                if let Some(u) = draw_city(rc, city) {
                    return u;
                }
            }
        }
        unit_ui::draw_units(rc, false);
        unit_ui::draw_units(rc, true);
    }
    StateUpdate::None
}

fn get_overlay(rc: &RenderContext) -> HashMap<Position, Terrain> {
    match &rc.state.active_dialog {
        ActiveDialog::ExploreResolution(r) => r
            .block
            .block
            .tiles(&r.block.position, r.rotation)
            .iter()
            .map(|(pos, t)| (*pos, t.clone()))
            .collect(),
        _ => HashMap::new(),
    }
}

pub fn pan_and_zoom(state: &mut State) {
    let (_, wheel) = mouse_wheel();
    let new_zoom = state.camera.zoom + wheel * 0.0001;
    let x = new_zoom.x;
    if x < 0.005 && x > 0.0005 {
        state.camera.zoom = new_zoom;
    }

    let pan_map = is_mouse_button_down(MouseButton::Left);
    if state.pan_map && pan_map {
        let mut new_offset = state
            .camera
            .offset
            .add(mouse_delta_position().mul(vec2(-1., 1.)));
        let min = MIN_OFFSET * state.camera.zoom / ZOOM;
        if new_offset.x < min.x {
            new_offset.x = min.x;
        }
        if new_offset.y < min.y {
            new_offset.y = min.y;
        }
        let max = MAX_OFFSET * state.camera.zoom / ZOOM;
        if new_offset.x > max.x {
            new_offset.x = max.x;
        }
        if new_offset.y > max.y {
            new_offset.y = max.y;
        }
        state.camera.offset = new_offset;
    }
    state.pan_map = pan_map;
}

fn overlay_color(rc: &RenderContext, pos: Position) -> Color {
    let state = &rc.state;

    match &state.active_dialog {
        ActiveDialog::MoveUnits(s) => {
            if let Some(start) = s.start {
                if start == pos {
                    alpha_overlay(0.5)
                } else if s
                    .destinations
                    .list
                    .iter()
                    .any(|d| matches!(d, MoveDestination::Tile(p, _) if *p == pos))
                {
                    MOVE_DESTINATION
                } else {
                    alpha_overlay(0.)
                }
            } else {
                alpha_overlay(0.)
            }
        }
        ActiveDialog::PositionRequest(r) => {
            if r.selected.contains(&pos) {
                with_alpha(HighlightType::Primary.color(), 0.5)
            } else if r.request.choices.contains(&pos) {
                with_alpha(HighlightType::Choices.color(), 0.5)
            } else {
                alpha_overlay(0.)
            }
        }
        _ => {
            if let Some(p) = state.focused_tile {
                highlight_if(p == pos)
            } else {
                alpha_overlay(0.)
            }
        }
    }
}

fn alpha_overlay(alpha: f32) -> Color {
    with_alpha(WHITE, alpha)
}

fn with_alpha(base: Color, alpha: f32) -> Color {
    let mut v = base.to_vec();
    v.w = alpha;
    Color::from_vec(v)
}

fn draw_combat_arrow(c: &CombatStats) {
    let from = hex_ui::center(c.attacker.position);
    let to = hex_ui::center(c.defender.position);
    let to_vec = vec2(to.x, to.y);
    let from_vec = vec2(from.x, from.y);
    let end = from_vec.add(to_vec.sub(from_vec).mul(0.7));
    draw_line(from.x, from.y, end.x, end.y, 10., BLACK);
    let angle = from_vec.sub(to_vec).normalize();
    draw_triangle(
        to_vec.add(angle.rotate(vec2(10., 0.))),
        to_vec.add(angle.rotate(vec2(30., 30.))),
        to_vec.add(angle.rotate(vec2(30., -30.))),
        BLACK,
    );
}

fn highlight_if(b: bool) -> Color {
    alpha_overlay(if b { 0.5 } else { 0. })
}

pub fn show_tile_menu(rc: &RenderContext, pos: Position) -> StateUpdate {
    if let Some(city) = rc.game.try_get_any_city(pos) {
        if rc.can_control_shown_player() && rc.shown_player.index == city.player_index {
            return show_city_menu(rc, city);
        }
    }

    let mut icons = move_units_buttons(rc, pos);
    if let Some(action) = found_city_button(rc, pos) {
        icons.push(action);
    }

    show_map_action_buttons(rc, &icons)
}

fn found_city_button<'a>(rc: &'a RenderContext<'a>, pos: Position) -> Option<IconAction<'a>> {
    let game = rc.game;

    unit_ui::units_on_tile(game, pos).find_map(|(_index, unit)| {
        (unit.can_found_city(game)
            && rc.can_play_action_for_player(&PlayingActionType::FoundCity, unit.player_index))
        .then_some(IconAction::new(
            rc.assets().unit(UnitType::Settler, rc.shown_player),
            vec!["Found a new city".to_string()],
            Box::new(move || {
                StateUpdate::execute(Action::Playing(PlayingAction::FoundCity {
                    settler: unit.id,
                }))
            }),
        ))
    })
}

pub fn move_units_button<'a>(
    rc: &'a RenderContext,
    pos: Position,
    move_intent: MoveIntent,
) -> Option<IconAction<'a>> {
    if !rc.can_play_action(&PlayingActionType::MoveUnits)
        || movable_units(pos, rc.game, rc.shown_player, move_intent.to_predicate()).is_empty()
    {
        return None;
    }
    Some(IconAction::new(
        move_intent.icon(rc),
        vec![move_intent.toolip().to_string()],
        Box::new(move || StateUpdate::move_units(rc, Some(pos), move_intent)),
    ))
}

pub fn move_units_buttons<'a>(rc: &'a RenderContext, pos: Position) -> Vec<IconAction<'a>> {
    let mut res = vec![];
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Land) {
        res.push(action);
    }
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Sea) {
        res.push(action);
    }
    if let Some(action) = move_units_button(rc, pos, MoveIntent::Disembark) {
        res.push(action);
    }
    res
}

pub fn show_map_action_buttons(rc: &RenderContext, icons: &IconActionVec) -> StateUpdate {
    for pass in 0..2 {
        for (i, icon) in icons.iter().enumerate() {
            let p = icon_pos(-(icons.len() as i8) / 2 + i as i8, -1);
            let center = bottom_center_anchor(rc) + p + vec2(15., 15.);
            let radius = 20.;
            if pass == 0 {
                if icon.warning {
                    draw_circle(center.x, center.y, radius, RED);
                }
                if bottom_center_texture(rc, icon.texture, p, "") {
                    return (icon.action)();
                }
            } else {
                show_tooltip_for_circle(rc, &icon.tooltip, center, radius);
            }
        }
    }
    StateUpdate::None
}

pub fn explore_dialog(rc: &RenderContext, r: &ExploreResolutionConfig) -> StateUpdate {
    if ok_button(
        rc,
        OkTooltip::Valid("Accept current tile rotation".to_string()),
    ) {
        return StateUpdate::execute(Action::Response(EventResponse::ExploreResolution(
            r.rotation,
        )));
    }
    if bottom_right_texture(
        rc,
        &rc.assets().rotate_explore,
        cancel_button_pos(),
        "Rotate tile",
    ) {
        let mut new = r.clone();
        new.rotation = (r.rotation + 3).rem(6);
        return StateUpdate::OpenDialog(ActiveDialog::ExploreResolution(new));
    }

    StateUpdate::None
}
