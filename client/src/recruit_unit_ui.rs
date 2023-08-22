use macroquad::prelude::*;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{Units, UnitType};

use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::dialog_ui::active_dialog_window;
use crate::ui_state::{ActiveDialog, StateUpdate, StateUpdates};
use crate::unit_ui;

#[derive(Clone)]
pub struct RecruitUnitSelection {
    player_index: usize,
    city_position: Position,
    pub units: Vec<UnitType>,
    available_units: Units,
    pub leader_index: Option<usize>,
    pub replaced_units: Vec<u32>,
}

impl RecruitUnitSelection {
    pub fn new(
        player_index: usize,
        city_position: Position,
        available_units: Units,
    ) -> RecruitUnitSelection {
        RecruitUnitSelection {
            player_index,
            city_position,
            units: vec![],
            leader_index: None,
            replaced_units: vec![],
            available_units,
        }
    }
}

pub fn select_dialog(game: &Game, sel: &RecruitUnitSelection) -> StateUpdate {
    let mut updates = StateUpdates::new();
    active_dialog_window(|ui| {
        let player = game.get_player(sel.player_index);

        ui.label(None, &selection_label(sel, player));

        for (unit_type, name) in unit_ui::non_leader_names() {

                //todo leader

                for (label, new) in new_selections(game,sel, &unit_type, name) {
                    if player.can_recruit(
                        &new.units,
                        new.city_position,
                        new.leader_index,
                        &new.replaced_units,
                    ) && ui.button(None, label)
                    {
                        updates.add(StateUpdate::SetDialog(ActiveDialog::RecruitUnitSelection(
                            new,
                        )));
                    };
                }
            }

        if ui.button(Vec2::new(320., 20.), "OK") {
            updates.add(StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                game,
                sel.player_index,
                sel.city_position,
                ConstructionProject::Units(sel.clone()),
            ))));
        };
        if ui.button(Vec2::new(320., 40.), "Cancel") {
            updates.add(StateUpdate::Cancel);
        };
    });
    updates.result()
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
