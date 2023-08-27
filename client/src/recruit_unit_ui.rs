use macroquad::prelude::*;
use server::game::Game;
use server::position::Position;
use server::unit::{UnitType, Units};

use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::select_ui::{CountSelector, HasCountSelectableObject};
use crate::ui_state::{ActiveDialog, StateUpdate};
use crate::{select_ui, unit_ui};

#[derive(Clone)]
pub struct SelectableUnit {
    pub unit_type: UnitType,
    pub selectable: CountSelector,
    name: String,
}

#[derive(Clone)]
pub struct RecruitAmount {
    player_index: usize,
    city_position: Position,
    pub units: Units,
    pub leader_index: Option<usize>,
    pub selectable: Vec<SelectableUnit>,
}

impl HasCountSelectableObject for SelectableUnit {
    fn counter(&self) -> &CountSelector {
        &self.selectable
    }
    fn counter_mut(&mut self) -> &mut CountSelector {
        &mut self.selectable
    }
}

impl RecruitAmount {
    pub fn new_selection(
        game: &Game,
        player_index: usize,
        city_position: Position,
        units: Units,
        must_show_units: &[SelectableUnit],
    ) -> StateUpdate {
        let player = game.get_player(player_index);
        let selectable: Vec<SelectableUnit> = unit_ui::non_leader_names()
            .iter()
            .filter_map(|(unit_type, name)| {
                let mut all = units.clone();
                all += unit_type;

                let current = units.get(unit_type);
                let max = if player.can_recruit_without_replaced(
                    all.to_vec().as_slice(),
                    city_position,
                    None,
                ) {
                    u32::from(current + 1)
                } else {
                    u32::from(current)
                };
                if max == 0 && !must_show_units.iter().any(|u| &u.unit_type == unit_type) {
                    None
                } else {
                    Some(SelectableUnit {
                        name: (*name).to_string(),
                        unit_type: unit_type.clone(),
                        selectable: CountSelector {
                            current: u32::from(current),
                            min: 0,
                            max,
                        },
                    })
                }
            })
            .collect();

        StateUpdate::SetDialog(ActiveDialog::RecruitUnitSelection(RecruitAmount {
            player_index,
            city_position,
            units,
            leader_index: None,
            selectable,
        }))
    }
}

#[derive(Clone)]
#[allow(dead_code)] //todo(Gregor)
pub struct RecruitSelection {
    pub selection: RecruitAmount,
    available_units: Units,
    pub replaced_units: Vec<u32>,
}

impl RecruitSelection {
    pub fn new(selection: RecruitAmount, available_units: Units) -> RecruitSelection {
        RecruitSelection {
            selection,
            replaced_units: vec![],
            available_units,
        }
    }
}

pub fn select_dialog(game: &Game, sel: &RecruitAmount) -> StateUpdate {
    select_ui::count_dialog(
        sel,
        |s| s.selectable.clone(),
        |s| s.name.clone(),
        |_s| true,
        |s| {
            //todo(Gregor) check if replace is needed
            // StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(ReplaceUnits::new(
            //     s.clone(),
            //     game.get_player(s.player_index).available_units.clone(),
            // )))

            StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                game,
                s.player_index,
                s.city_position,
                ConstructionProject::Units(RecruitSelection::new(
                    s.clone(),
                    game.get_player(s.player_index).available_units.clone(),
                )),
            )))
        },
        |_s, _u| true,
        |s, u| {
            let mut units = s.units.clone();
            units += &u.unit_type;
            update_selection(game, s, units)
        },
        |s, u| {
            let mut units = s.units.clone();
            units -= &u.unit_type;
            update_selection(game, s, units)
        },
    )

    //todo(Gregor) leader
}

fn update_selection(game: &Game, s: &RecruitAmount, units: Units) -> StateUpdate {
    RecruitAmount::new_selection(
        game,
        s.player_index,
        s.city_position,
        units,
        s.selectable.as_slice(),
    )
}

// fn new_selections(game: &Game, sel: &RecruitUnitSelection, unit_type: &UnitType, name: &str) -> Vec<(String, RecruitUnitSelection)> {
//     let mut res = vec![];
//     if sel.available_units.has_unit(unit_type) {
//         let mut new = sel.clone();
//         new.units.push(unit_type.clone());
//         new.available_units -= unit_type;
//         res.push((format!("Add {name}"), new));
//     } else {
//         let p = game.get_player(sel.player_index);
//         for rep in p.units.iter().filter(|u| &u.unit_type == unit_type).collect::<Vec<_>>() {
//             let mut new = sel.clone();
//             new.units.push(unit_type.clone());
//             new.replaced_units.push(rep.id);
//             res.push((format!("Add {} (Replace {})", name, unit_ui::label(rep)), new));
//         }
//     }
//     res
// }
//
// fn selection_label(sel: &RecruitUnitSelection, player: &Player) -> String {
//     let names = sel
//         .units
//         .iter()
//         .map(unit_ui::name)
//         .collect::<Vec<&str>>()
//         .join(", ");
//
//     let replaced = sel
//         .replaced_units
//         .iter()
//         .map(|id| {
//             unit_ui::label(player.get_unit(*id).unwrap())
//         })
//         .collect::<Vec<String>>()
//         .join(", ");
//     let replaced = if replaced.is_empty() {
//         String::new()
//     } else {
//         format!(" Replaced: {replaced}")
//     };
//
//     format!("Units: {names}{replaced}")
// }
