use macroquad::prelude::*;

use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{Unit, UnitType, Units};

use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::select_ui::{CountSelector, HasCountSelectableObject};
use crate::ui_state::{ActiveDialog, StateUpdate};
use crate::unit_ui::{UnitSelection, UnitSelectionConfirm};
use crate::{select_ui, unit_ui};

#[derive(Clone)]
pub struct SelectableUnit {
    pub unit_type: UnitType,
    pub selectable: CountSelector,
    name: String,
    leader_index: Option<usize>,
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
        leader_index: Option<usize>,
        must_show_units: &[SelectableUnit],
    ) -> StateUpdate {
        let player = game.get_player(player_index);
        let selectable: Vec<SelectableUnit> = new_units(player)
            .into_iter()
            .filter_map(|u| {
                selectable_unit(
                    city_position,
                    &units,
                    leader_index,
                    must_show_units,
                    player,
                    &u,
                )
            })
            .collect();

        StateUpdate::SetDialog(ActiveDialog::RecruitUnitSelection(RecruitAmount {
            player_index,
            city_position,
            units,
            leader_index,
            selectable,
        }))
    }
}

fn selectable_unit(
    city_position: Position,
    units: &Units,
    leader_index: Option<usize>,
    must_show_units: &[SelectableUnit],
    player: &Player,
    unit: &NewUnit,
) -> Option<SelectableUnit> {
    let mut all = units.clone();
    all += &unit.unit_type;

    let current: u8 = if matches!(unit.unit_type, UnitType::Leader) {
        u8::from(leader_index.is_some_and(|i| i == unit.leader_index.unwrap()))
    } else {
        units.get(&unit.unit_type)
    };

    let max = if player.can_recruit_without_replaced(
        all.to_vec().as_slice(),
        city_position,
        unit.leader_index.or(leader_index),
    ) {
        u32::from(current + 1)
    } else {
        u32::from(current)
    };
    if max == 0
        && !must_show_units
            .iter()
            .any(|u| u.unit_type == unit.unit_type)
    {
        None
    } else {
        Some(SelectableUnit {
            name: unit.name.to_string(),
            unit_type: unit.unit_type.clone(),
            selectable: CountSelector {
                current: u32::from(current),
                min: 0,
                max,
            },
            leader_index: unit.leader_index,
        })
    }
}

struct NewUnit {
    unit_type: UnitType,
    name: String,
    leader_index: Option<usize>,
}

impl NewUnit {
    fn new(unit_type: UnitType, name: &str, leader_index: Option<usize>) -> NewUnit {
        NewUnit {
            unit_type,
            name: name.to_string(),
            leader_index,
        }
    }
}

fn new_units(player: &Player) -> Vec<NewUnit> {
    unit_ui::non_leader_names()
        .into_iter()
        .map(|(u, n)| NewUnit::new(u, n, None::<usize>))
        .chain(
            player
                .available_leaders
                .iter()
                .enumerate()
                .map(|(i, l)| NewUnit::new(UnitType::Leader, l.name.as_str(), Some(i))),
        )
        .collect()
}

#[derive(Clone)]
pub struct RecruitSelection {
    pub amount: RecruitAmount,
    pub available_units: Units,
    pub need_replacement: Units,
    pub replaced_units: Vec<u32>,
    pub current_city: Option<Position>,
}

impl RecruitSelection {
    pub fn new(game: &Game, amount: RecruitAmount, replaced_units: Vec<u32>) -> RecruitSelection {
        let available_units = game.get_player(amount.player_index).available_units.clone();
        let need_replacement = available_units.get_units_to_replace(&amount.units);

        RecruitSelection {
            amount,
            available_units,
            need_replacement,
            replaced_units,
            current_city: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.need_replacement.is_empty()
    }
}

impl UnitSelection for RecruitSelection {
    fn selected_units(&self) -> &[u32] {
        &self.replaced_units
    }

    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.replaced_units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.need_replacement.has_unit(&unit.unit_type)
    }

    fn current_tile(&self) -> Option<Position> {
        self.current_city
    }

    fn confirm(&self, game: &Game) -> UnitSelectionConfirm {
        if game.get_player(self.amount.player_index).can_recruit(
            self.amount.units.clone().to_vec().as_slice(),
            self.amount.city_position,
            self.amount.leader_index,
            self.replaced_units.as_slice(),
        ) {
            UnitSelectionConfirm::Valid
        } else {
            UnitSelectionConfirm::Invalid
        }
    }
}

pub fn select_dialog(game: &Game, a: &RecruitAmount) -> StateUpdate {
    select_ui::count_dialog(
        a,
        |s| s.selectable.clone(),
        |s| s.name.as_ref(),
        |_s| true,
        |amount| {
            let sel = RecruitSelection::new(game, amount.clone(), vec![]);

            if sel.is_finished() {
                StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                    game,
                    amount.player_index,
                    amount.city_position,
                    ConstructionProject::Units(sel),
                )))
            } else {
                StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(sel))
            }
        },
        |_s, _u| true,
        |s, u| {
            let mut units = s.units.clone();
            units += &u.unit_type;
            update_selection(game, s, units, u.leader_index.or(s.leader_index))
        },
        |s, u| {
            let mut units = s.units.clone();
            units -= &u.unit_type;
            update_selection(
                game,
                s,
                units,
                if matches!(u.unit_type, UnitType::Leader) {
                    None
                } else {
                    s.leader_index
                },
            )
        },
    )
}

fn update_selection(
    game: &Game,
    s: &RecruitAmount,
    units: Units,
    leader_index: Option<usize>,
) -> StateUpdate {
    RecruitAmount::new_selection(
        game,
        s.player_index,
        s.city_position,
        units,
        leader_index,
        s.selectable.as_slice(),
    )
}

pub fn replace_dialog(game: &Game, sel: &RecruitSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RecruitSelection>(
        game,
        sel,
        |new| StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(new.clone())),
        |new: RecruitSelection| {
            StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
                game,
                new.amount.player_index,
                new.amount.city_position,
                ConstructionProject::Units(new),
            )))
        },
    )
}

pub fn click_replace(pos: Position, s: &RecruitSelection) -> StateUpdate {
    let mut new = s.clone();
    new.current_city = Some(pos);
    StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(new))
}
