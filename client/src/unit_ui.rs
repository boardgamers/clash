use macroquad::color::BLACK;
use macroquad::math::{u32, vec2, Vec2};
use macroquad::prelude::{draw_text, WHITE};
use macroquad::shapes::draw_circle;
use macroquad::ui::Ui;

use server::game::Game;
use server::position::Position;
use server::unit::{Unit, UnitType};

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::dialog_ui::active_dialog_window;
use crate::select_ui::{confirm_update, ConfirmSelection};
use crate::{hex_ui, player_ui};

use crate::hex_ui::Point;
use itertools::Itertools;
use server::player::Player;

pub fn draw_unit(unit: &Unit, index: u32, selected: bool) {
    let p = unit_circle(index, unit.position);
    draw_circle(p.x, p.y, 11.0, if selected { WHITE } else { BLACK });
    draw_circle(p.x, p.y, 9.0, player_ui::player_color(unit.player_index));
    draw_text(unit_symbol(unit), p.x - 5.0, p.y + 5.0, 20.0, BLACK);
}

fn unit_circle(index: u32, position: Position) -> Point {
    let r = 40.0;
    hex_ui::rotate_around(hex_ui::center(position), r, (40 * index) as i32 + 45)
}

pub fn clicked_unit(pos: Position, mouse_pos: Vec2, player: &Player) -> Option<u32> {
    player
        .units
        .iter()
        .filter(|u| u.position == pos)
        .enumerate()
        .find_map(|(i, u)| {
            let p = unit_circle(i.try_into().unwrap(), pos);
            let d = vec2(p.x - mouse_pos.x, p.y - mouse_pos.y);
            if d.length() <= 11.0 {
                Some(u.id)
            } else {
                None
            }
        })
}

fn unit_symbol(unit: &Unit) -> &str {
    match unit.unit_type {
        UnitType::Infantry => "I",
        UnitType::Cavalry => "C",
        UnitType::Elephant => "E",
        UnitType::Leader => "L",
        UnitType::Ship => "P",
        UnitType::Settler => "S",
    }
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

pub fn draw_units(game: &Game, state: &State) {
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
            let selected = *p == game.active_player() && selected_units.contains(&u.id);
            draw_unit(u, i.try_into().unwrap(), selected);
        });
    }
}

pub trait UnitSelection: ConfirmSelection {
    fn selected_units(&self) -> &[u32];
    fn selected_units_mut(&mut self) -> &mut Vec<u32>;
    fn can_select(&self, game: &Game, unit: &Unit) -> bool;
    fn current_tile(&self) -> Option<Position>;
}

pub fn unit_selection_dialog<T: UnitSelection>(
    game: &Game,
    player: &ShownPlayer,
    title: &str,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
    on_ok: impl FnOnce(T) -> StateUpdate,
    additional: impl FnOnce(&mut Ui) -> StateUpdate,
) -> StateUpdate {
    if let Some(current_tile) = sel.current_tile() {
        active_dialog_window(player, title, |ui| {
            for (i, (p, unit_id)) in units_on_tile(game, current_tile).enumerate() {
                let unit = game.get_player(p).get_unit(unit_id).unwrap();
                let can_sel = sel.can_select(game, unit);
                let is_selected = sel.selected_units().contains(&unit_id);
                let mut l = label(unit);
                if is_selected {
                    l += " (selected)";
                }

                let pos = vec2(((i / 4) as f32) * 200., i.rem_euclid(4) as f32 * 35.);
                if !can_sel {
                    ui.label(pos, &l);
                } else if ui.button(pos, l) {
                    let mut new = sel.clone();
                    if is_selected {
                        new.selected_units_mut().retain(|u| u != &unit_id);
                    } else {
                        new.selected_units_mut().push(unit_id);
                    }
                    return on_change(new);
                }
            }
            confirm_update(sel, player, || on_ok(sel.clone()), ui, &sel.confirm(game))
                .or(|| additional(ui))
        })
    } else {
        StateUpdate::None
    }
}

pub fn units_on_tile(game: &Game, pos: Position) -> impl Iterator<Item = (usize, u32)> + '_ {
    game.players.iter().flat_map(move |p| {
        p.units.iter().filter_map(move |unit| {
            if unit.position == pos {
                Some((p.index, unit.id))
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

pub fn label(unit: &Unit) -> String {
    let name = name(&unit.unit_type);
    let res = if !unit.can_move() {
        " (can't move) "
    } else if !unit.can_attack() && !unit.unit_type.is_settler() {
        " (can't attack) "
    } else {
        ""
    };

    format!("{name}{res}")
}
