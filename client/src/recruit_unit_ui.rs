use std::iter;
use macroquad::prelude::*;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{Units, UnitType};

use crate::payment_ui::{HasSelectableObject, select_count_dialog, SelectableObject};
use crate::ui_state::{ActiveDialog, StateUpdate};
use crate::unit_ui;

#[derive(Clone)]
pub struct SelectableUnit {
    pub unit_type: UnitType,
    pub selectable: SelectableObject,
    name : String,
}

#[derive(Clone)]
pub struct RecruitUnitSelection {
    player_index: usize,
    city_position: Position,
    pub units: Units,
    pub leader_index: Option<usize>,
    pub selectable: Vec<SelectableUnit>,
}

impl HasSelectableObject for SelectableUnit {
    fn counter(&self) -> &SelectableObject {
        &self.selectable
    }
}

impl RecruitUnitSelection {
    pub fn new(
        game: &Game,
        player_index: usize,
        city_position: Position,
        units: Units,
    ) -> StateUpdate {
        let player = game.get_player(player_index);
        let x: Vec<SelectableUnit> = unit_ui::non_leader_names().iter().flat_map(|(unit_type, name)| {
            let mut all = units.clone();
            all += &unit_type;

            let iterator = all.into_iter();
            let vec = iterator.flat_map(| (u, c)| {iter::repeat(u).take(c as usize)}).collect();
            if player.can_recruit_without_replaced(vec, city_position, None) {
                let current = units.get(unit_type);
                Some(
                    SelectableUnit {
                        name: name.to_string(),
                        unit_type: unit_type.clone(),
                        selectable: SelectableObject {
                            current: current as u32,
                            min: 0,
                            max: (current + 1) as u32,
                        },
                    },
                )
            } else {
                None
            }
        }).collect();

        StateUpdate::SetDialog(ActiveDialog::RecruitUnitSelection(
            RecruitUnitSelection {
                player_index,
                city_position,
                units: Units::empty(),
                leader_index: None,
            }))
    }
}

#[derive(Clone)]
pub struct ReplaceUnits {
    pub selection: RecruitUnitSelection,
    available_units: Units,
    pub replaced_units: Vec<u32>,
}

impl ReplaceUnits {
    pub fn new(
        selection: RecruitUnitSelection,
        available_units: Units,
    ) -> ReplaceUnits {
        ReplaceUnits {
            selection,
            replaced_units: vec![],
            available_units,
        }
    }
}

pub fn select_dialog(game: &Game, sel: &RecruitUnitSelection) -> StateUpdate {
    select_count_dialog(
        sel,
        |s| unit_ui::non_leader_names().to_vec(),
        |(unit_type, name)| { name.to_string() },
        |s| true,
        |s| StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(ReplaceUnits::new(
            s.clone(),
            game.get_player(s.player_index).available_units.clone(),
        ))),
        |s, u| true,
        |s, u| {
            let mut units = s.units.clone();
            units += &(*u).0;
            RecruitUnitSelection::new(
                game,
                s.player_index,
                s.city_position,
                units,
            )
        },
        |s, u| {
            let mut units = s.units.clone();
            units -= &(*u).0;
            RecruitUnitSelection::new(
                game,
                s.player_index,
                s.city_position,
                units,
            )
        },
    )

    //todo leader
}

fn new_selections(game: &Game, sel: &RecruitUnitSelection, unit_type: &UnitType, name: &str) -> Vec<(String, RecruitUnitSelection)> {
    let mut res = vec![];
    if sel.available_units.has_unit(unit_type) {
        let mut new = sel.clone();
        new.units.push(unit_type.clone());
        new.available_units -= unit_type;
        res.push((format!("Add {name}"), new));
    } else {
        let p = game.get_player(sel.player_index);
        for rep in p.units.iter().filter(|u| &u.unit_type == unit_type).collect::<Vec<_>>() {
            let mut new = sel.clone();
            new.units.push(unit_type.clone());
            new.replaced_units.push(rep.id);
            res.push((format!("Add {} (Replace {})", name, unit_ui::label(rep)), new));
        }
    }
    res
}

fn selection_label(sel: &RecruitUnitSelection, player: &Player) -> String {
    let names = sel
        .units
        .iter()
        .map(unit_ui::name)
        .collect::<Vec<&str>>()
        .join(", ");

    let replaced = sel
        .replaced_units
        .iter()
        .map(|id| {
            unit_ui::label(player.get_unit(*id).unwrap())
        })
        .collect::<Vec<String>>()
        .join(", ");
    let replaced = if replaced.is_empty() {
        String::new()
    } else {
        format!(" Replaced: {replaced}")
    };

    format!("Units: {names}{replaced}")
}
