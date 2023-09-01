use macroquad::prelude::*;
use server::game::Game;
use server::position::Position;
use server::unit::{UnitType, Units, Unit};

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
        if game.get_player(self.amount.player_index)
            .can_recruit(
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
        |s| s.name.clone(),
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

pub fn replace_dialog(game: &Game, sel: &RecruitSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RecruitSelection>(
        game,
        sel,
        |new| {
            StateUpdate::SetDialog(ActiveDialog::ReplaceUnits(new.clone()))
        },
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
