use macroquad::prelude::*;

use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{Unit, UnitType, Units};

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::dialog_ui::OkTooltip;
use crate::hex_ui::Point;
use crate::render_context::RenderContext;
use crate::select_ui::{ConfirmSelection, CountSelector, HasCountSelectableObject};
use crate::unit_ui::{draw_unit_type, UnitSelection};
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

        StateUpdate::OpenDialog(ActiveDialog::RecruitUnitSelection(RecruitAmount {
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
        }
    }

    pub fn is_finished(&self) -> bool {
        self.need_replacement.is_empty()
    }
}

impl UnitSelection for RecruitSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.replaced_units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.need_replacement.has_unit(&unit.unit_type)
    }
}

impl ConfirmSelection for RecruitSelection {
    fn confirm(&self, game: &Game) -> OkTooltip {
        if game.get_player(self.amount.player_index).can_recruit(
            self.amount.units.clone().to_vec().as_slice(),
            self.amount.city_position,
            self.amount.leader_index,
            self.replaced_units.as_slice(),
        ) {
            OkTooltip::Valid("Recruit units".to_string())
        } else {
            OkTooltip::Invalid("Replace exact amount of units".to_string())
        }
    }
}

pub fn select_dialog(rc: &RenderContext, a: &RecruitAmount) -> StateUpdate {
    let game = rc.game;
    select_ui::count_dialog(
        rc,
        a,
        |s| s.selectable.clone(),
        |s, p| {
            draw_unit_type(
                rc,
                false,
                Point::from_vec2(p),
                &s.unit_type,
                rc.shown_player.index,
                &format!(
                    "{} ({} available with current resources)",
                    s.name, s.selectable.max
                ),
                20.,
            );
        },
        |_s| OkTooltip::Valid("Recruit units".to_string()),
        || {
            let sel = RecruitSelection::new(game, a.clone(), vec![]);

            if sel.is_finished() {
                StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(
                    ConstructionPayment::new(
                        rc,
                        game.get_city(a.player_index, a.city_position),
                        "units",
                        ConstructionProject::Units(sel),
                    ),
                ))
            } else {
                StateUpdate::OpenDialog(ActiveDialog::ReplaceUnits(sel))
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

pub fn replace_dialog(rc: &RenderContext, sel: &RecruitSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RecruitSelection>(rc, sel, |new: RecruitSelection| {
        StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
            rc,
            rc.game
                .get_city(new.amount.player_index, new.amount.city_position),
            "units",
            ConstructionProject::Units(new),
        )))
    })
}
