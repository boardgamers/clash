use macroquad::color::BLACK;
use macroquad::math::{u32, vec2, Vec2};
use macroquad::prelude::WHITE;
use macroquad::shapes::draw_circle;

use server::game::Game;
use server::position::Position;
use server::unit::{carried_units, Unit, UnitType};

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::select_ui::ConfirmSelection;
use crate::{hex_ui, player_ui};

use crate::dialog_ui::{cancel_button_with_tooltip, ok_button};
use crate::hex_ui::Point;
use crate::layout_ui::{draw_scaled_icon, is_in_circle};
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::player::Player;

pub struct UnitPlace {
    pub center: Point,
    pub radius: f32,
}

impl UnitPlace {
    pub fn new(center: Point, radius: f32) -> UnitPlace {
        UnitPlace { center, radius }
    }
}

pub fn draw_unit_type(
    rc: &RenderContext,
    selected: bool,
    center: Point,
    unit_type: &UnitType,
    player_index: usize,
    tooltip: &str,
    size: f32,
) {
    draw_circle(
        center.x,
        center.y,
        size,
        if selected { WHITE } else { BLACK },
    );
    draw_circle(
        center.x,
        center.y,
        size - 2.,
        player_ui::player_color(player_index),
    );
    let icon_size = size * 1.1;
    draw_scaled_icon(
        rc,
        &rc.assets().units[unit_type],
        tooltip,
        vec2(center.x - icon_size / 2., center.y - icon_size / 2.),
        icon_size,
    );
}

fn carried_unit_place(carrier: &UnitPlace, index: usize) -> UnitPlace {
    let r = carrier.radius / 2.0;
    UnitPlace::new(hex_ui::rotate_around(carrier.center, r, 180 * index), r)
}

fn unit_place(rc: &RenderContext, index: usize, position: Position) -> UnitPlace {
    let has_city = rc.game.get_any_city(position).is_some();
    let c = hex_ui::center(position);
    let n = units_on_tile(rc.game, position)
        .filter(|(_, u)| !u.is_transported())
        .count();
    if has_city || n > 4 {
        UnitPlace::new(hex_ui::rotate_around(c, 40.0, (40 * index) + 45), 11.0)
    } else if n == 1 {
        UnitPlace::new(c, 18.0)
    } else if n == 2 {
        UnitPlace::new(hex_ui::rotate_around(c, 27.0, 180 * index), 18.0)
    } else {
        UnitPlace::new(hex_ui::rotate_around(c, 27.0, 90 * index), 18.0)
    }
}

pub fn click_unit(
    rc: &RenderContext,
    pos: Position,
    mouse_pos: Vec2,
    player: &Player,
) -> Option<u32> {
    player
        .units
        .iter()
        .filter(|u| u.position == pos && !u.is_transported())
        .enumerate()
        .find_map(|(i, u)| {
            let place = unit_place(rc, i, pos);
            let carried_units = carried_units(rc.game, player.index, u.id);
            for (j, carried) in carried_units.iter().enumerate() {
                let carried_place = carried_unit_place(&place, j);
                if is_in_circle(mouse_pos, carried_place.center, carried_place.radius) {
                    return Some(*carried);
                }
            }
            if is_in_circle(mouse_pos, place.center, place.radius) {
                Some(u.id)
            } else {
                None
            }
        })
}

pub fn non_leader_names() -> [(UnitType, &'static str); 5] {
    [
        (UnitType::Settler, "Settler"),
        (UnitType::Infantry, "Infantry"),
        (UnitType::Ship, "Ship"),
        (UnitType::Elephant, "Elephant"),
        (UnitType::Cavalry, "Cavalry"),
    ]
}

pub fn draw_units(rc: &RenderContext, tooltip: bool) {
    let selected_units = match rc.state.active_dialog {
        ActiveDialog::MoveUnits(ref s) => s.units.clone(),
        ActiveDialog::ReplaceUnits(ref s) => s.replaced_units.clone(),
        ActiveDialog::RemoveCasualties(ref s) => s.units.clone(),
        _ => vec![],
    };

    for (_pos, on_tile) in &rc
        .game
        .players
        .iter()
        .flat_map(|p| {
            p.units
                .iter()
                .filter(|u| !u.is_transported())
                .map(move |u| (p.index, u))
        })
        .sorted_by_key(|(_, u)| u.position)
        .chunk_by(|(_, a)| a.position)
    {
        on_tile
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
            .for_each(|(i, (p, u))| {
                let place = unit_place(rc, i, u.position);

                draw_unit(rc, tooltip, &selected_units, *p, u, &place);

                let player = rc.game.get_player(*p);
                let carried = carried_units(rc.game, *p, u.id);
                carried.iter().enumerate().for_each(|(j, u)| {
                    draw_unit(
                        rc,
                        tooltip,
                        &selected_units,
                        *p,
                        player.get_unit(*u).unwrap(),
                        &carried_unit_place(&place, j),
                    );
                });
            });
    }
}

fn draw_unit(
    rc: &RenderContext,
    tooltip: bool,
    selected_units: &[u32],
    player_index: usize,
    unit: &Unit,
    place: &UnitPlace,
) {
    let center = place.center;
    let radius = place.radius;
    let game = &rc.game;
    if tooltip {
        let army_move = game
            .get_player(player_index)
            .has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
        show_tooltip_for_circle(rc, &unit_label(unit, army_move), center.to_vec2(), radius);
    } else {
        let selected = player_index == game.active_player() && selected_units.contains(&unit.id);
        draw_unit_type(
            rc,
            selected,
            center,
            &unit.unit_type,
            unit.player_index,
            "",
            radius,
        );
    }
}

pub trait UnitSelection: ConfirmSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32>;
    fn can_select(&self, game: &Game, unit: &Unit) -> bool;
}

pub fn unit_selection_click<T: UnitSelection>(
    rc: &RenderContext,
    pos: Position,
    mouse_pos: Vec2,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
) -> StateUpdate {
    if let Some(unit_id) = click_unit(rc, pos, mouse_pos, rc.shown_player) {
        if sel.can_select(rc.game, rc.shown_player.get_unit(unit_id).unwrap()) {
            let mut new = sel.clone();
            unit_selection_clicked(unit_id, new.selected_units_mut());
            return on_change(new);
        }
    }
    StateUpdate::None
}

pub fn unit_selection_dialog<T: UnitSelection>(
    rc: &RenderContext,
    sel: &T,
    on_ok: impl FnOnce(T) -> StateUpdate,
) -> StateUpdate {
    if ok_button(rc, sel.confirm(rc.game)) {
        on_ok(sel.clone())
    } else {
        may_cancel(sel, rc)
    }
}

fn may_cancel(sel: &impl ConfirmSelection, rc: &RenderContext) -> StateUpdate {
    if let Some(cancel_name) = sel.cancel_name() {
        if cancel_button_with_tooltip(rc, cancel_name) {
            StateUpdate::Cancel
        } else {
            StateUpdate::None
        }
    } else {
        StateUpdate::None
    }
}

pub fn units_on_tile(game: &Game, pos: Position) -> impl Iterator<Item = (usize, Unit)> + '_ {
    game.players.iter().flat_map(move |p| {
        p.units.iter().filter_map(move |unit| {
            if unit.position == pos {
                Some((p.index, unit.clone()))
            } else {
                None
            }
        })
    })
}

pub fn name(u: &UnitType) -> &str {
    if let UnitType::Leader = u {
        return "Leader";
    }
    non_leader_names()
        .into_iter()
        .find(|(unit_type, _)| unit_type == u)
        .unwrap()
        .1
}

pub fn unit_label(unit: &Unit, army_move: bool) -> String {
    let name = name(&unit.unit_type);

    let res = if unit.unit_type.is_army_unit() && !army_move {
        " (research Tactics to move the unit) "
    } else if !unit.can_move() {
        " (can't move out of a Mountain this turn) "
    } else if !unit.can_attack() && !unit.unit_type.is_settler() {
        " (can't attack again this turn) "
    } else {
        ""
    };

    format!("{name}{res}")
}

pub fn unit_selection_clicked(unit_id: u32, units: &mut Vec<u32>) {
    if units.contains(&unit_id) {
        // deselect unit
        units.retain(|&id| id != unit_id);
    } else {
        units.push(unit_id);
    }
}
