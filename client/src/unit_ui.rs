use std::collections::{HashMap, HashSet};

use macroquad::math::u32;
use macroquad::prelude::draw_text;

use server::game::Game;
use server::position::Position;
use server::unit::{Unit, UnitType};

use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{StateUpdate, StateUpdates};
use crate::{hex_ui, player_ui};

pub fn draw_unit(unit: &Unit, index: u32) {
    let c = hex_ui::center(unit.position);
    let r = if unit.unit_type == UnitType::Settler {
        25.
    } else {
        40.
    };
    let p = hex_ui::rotate_around(c, r, (90 * index) as i32 + 45);
    draw_text(
        unit_symbol(unit),
        p.x - 7.0,
        p.y + 7.0,
        25.0,
        player_ui::player_color(unit.player_index),
    );
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

pub fn draw_units(game: &Game) {
    for p in &game.players {
        let mut positions: HashSet<&Position> = HashSet::new();
        let mut city_unit_index: HashMap<Position, u32> = HashMap::new();
        let mut settler_index: HashMap<Position, u32> = HashMap::new();
        for unit in &p.units {
            let map = if unit.unit_type == UnitType::Settler {
                &mut settler_index
            } else {
                &mut city_unit_index
            };
            let e = map.entry(unit.position).or_default();
            *e += 1;
            draw_unit(unit, *e);

            if positions.insert(&unit.position) {}
        }
    }
}

pub enum UnitSelectionConfirm {
    NoConfirm,
    Invalid,
    Valid,
}

pub trait UnitSelection: Clone {
    fn selected_units(&self) -> &[u32];
    fn selected_units_mut(&mut self) -> &mut Vec<u32>;
    fn can_select(&self, game: &Game, unit: &Unit) -> bool;
    fn current_tile(&self) -> Option<Position>;
    fn confirm(&self, game: &Game) -> UnitSelectionConfirm;
}

pub fn unit_selection_dialog<T: UnitSelection>(
    game: &Game,
    sel: &T,
    on_change: impl Fn(T) -> StateUpdate,
    on_ok: impl FnOnce(T) -> StateUpdate,
) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        if let Some(current_tile) = sel.current_tile() {
            for (p, unit_id) in units_on_tile(game, current_tile) {
                let unit = game.get_player(p).get_unit(unit_id).unwrap();
                let can_sel = sel.can_select(game, unit);
                let is_selected = sel.selected_units().contains(&unit_id);
                let mut l = label(unit);
                if is_selected {
                    l += " (selected)";
                }

                if !can_sel {
                    ui.label(None, &l);
                } else if ui.button(None, l) {
                    let mut new = sel.clone();
                    if is_selected {
                        new.selected_units_mut().retain(|u| u != &unit_id);
                    } else {
                        new.selected_units_mut().push(unit_id);
                    }
                    updates.add(on_change(new));
                }
            }
            let confirm = sel.confirm(game);
            match confirm {
                UnitSelectionConfirm::NoConfirm => {}
                UnitSelectionConfirm::Invalid => {
                    ui.label(None, "Invalid selection");
                    if ui.button(None, "Cancel") {
                        updates.add(StateUpdate::Cancel);
                    };
                }
                UnitSelectionConfirm::Valid => {
                    if ui.button(None, "OK") {
                        updates.add(on_ok(sel.clone()));
                    }
                    if ui.button(None, "Cancel") {
                        updates.add(StateUpdate::Cancel);
                    };
                }
            }
        } else {
            ui.label(None, "Select a starting tile");
        }
    });

    updates.result()
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
    } else if !unit.can_attack() {
        " (can't attack) "
    } else {
        ""
    };

    format!("{name}{res}")
}
