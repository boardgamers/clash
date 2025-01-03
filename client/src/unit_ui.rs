use macroquad::color::BLACK;
use macroquad::math::{u32, vec2, Vec2};
use macroquad::prelude::WHITE;
use macroquad::shapes::draw_circle;

use server::game::Game;
use server::position::Position;
use server::unit::{Unit, UnitType};

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::select_ui::{confirm_update, ConfirmSelection};
use crate::{hex_ui, player_ui};

use crate::hex_ui::Point;
use crate::layout_ui::{draw_scaled_icon, is_in_circle};
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::player::Player;

pub const UNIT_RADIUS: f32 = 11.0;

pub fn draw_unit_type(
    selected: bool,
    center: Point,
    unit_type: &UnitType,
    player_index: usize,
    state: &State,
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
        state,
        &state.assets.units[unit_type],
        tooltip,
        vec2(center.x - icon_size / 2., center.y - icon_size / 2.),
        icon_size,
    );
}

fn unit_center(index: u32, position: Position) -> Point {
    let r = 40.0;
    hex_ui::rotate_around(hex_ui::center(position), r, (40 * index) as i32 + 45)
}

pub fn unit_at_pos(pos: Position, mouse_pos: Vec2, player: &Player) -> Option<u32> {
    player
        .units
        .iter()
        .filter(|u| u.position == pos)
        .enumerate()
        .find_map(|(i, u)| {
            let p = unit_center(i.try_into().unwrap(), pos);
            if is_in_circle(mouse_pos, p, UNIT_RADIUS) {
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

pub fn draw_units(game: &Game, state: &State, tooltip: bool) {
    let selected_units = match state.active_dialog {
        ActiveDialog::MoveUnits(ref s) => s.units.clone(),
        ActiveDialog::ReplaceUnits(ref s) => s.replaced_units.clone(),
        ActiveDialog::RemoveCasualties(ref s) => s.units.clone(),
        _ => vec![],
    };

    for (_pos, units) in &game
        .players
        .iter()
        .flat_map(|p| p.units.iter().map(move |u| (p.index, u)))
        .sorted_by_key(|(_, u)| u.position)
        .chunk_by(|(_, a)| a.position)
    {
        let vec = units.collect::<Vec<_>>();
        vec.iter().enumerate().for_each(|(i, (p, u))| {
            if tooltip {
                let army_move = game
                    .get_player(*p)
                    .has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
                let point = unit_center(i.try_into().unwrap(), u.position);
                let center = vec2(point.x, point.y);
                show_tooltip_for_circle(state, &unit_label(u, army_move), center, UNIT_RADIUS);
            } else {
                let selected = *p == game.active_player() && selected_units.contains(&u.id);
                draw_unit_type(
                    selected,
                    unit_center(i.try_into().unwrap(), u.position),
                    &u.unit_type,
                    u.player_index,
                    state,
                    "",
                    UNIT_RADIUS,
                );
            }
        });
    }
}

pub trait UnitSelection: ConfirmSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32>;
    fn can_select(&self, game: &Game, unit: &Unit) -> bool;
}

pub fn unit_selection_click<T: UnitSelection>(
    game: &Game,
    player: &ShownPlayer,
    pos: Position,
    mouse_pos: Vec2,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
) -> StateUpdate {
    if let Some(unit_id) = unit_at_pos(pos, mouse_pos, player.get(game)) {
        let mut new = sel.clone();
        unit_selection_clicked(unit_id, new.selected_units_mut());
        return on_change(new);
    }
    StateUpdate::None
}

pub fn unit_selection_dialog<T: UnitSelection>(
    game: &Game,
    sel: &T,
    on_ok: impl FnOnce(T) -> StateUpdate,
    state: &State,
) -> StateUpdate {
    confirm_update(sel, || on_ok(sel.clone()), &sel.confirm(game), state)
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
