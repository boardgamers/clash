use itertools::Itertools;
use macroquad::prelude::*;

use crate::client_state::{ActiveDialog, StateUpdate};
use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::dialog_ui::{OkTooltip, cancel_button, ok_button};
use crate::render_context::RenderContext;
use crate::select_ui;
use crate::select_ui::{CountSelector, HasCountSelectableObject, HighlightType};
use crate::tooltip::show_tooltip_for_circle;
use crate::unit_ui::{UnitSelection, add_unit_description, draw_unit_type};
use server::game::Game;
use server::leader;
use server::player::{CostTrigger, Player};
use server::player_events::CostInfo;
use server::position::Position;
use server::recruit::{recruit_cost, recruit_cost_without_replaced};
use server::unit::UnitType::*;
use server::unit::{Unit, UnitType, Units, get_units_to_replace};
use server::utils::remove_element;

#[derive(Clone)]
pub struct SelectableUnit {
    pub unit_type: UnitType,
    pub selectable: CountSelector,
    cost: Result<CostInfo, String>,
}

#[derive(Clone)]
pub struct RecruitAmount {
    player_index: usize,
    city_position: Position,
    pub units: Units,
    pub selectable: Vec<SelectableUnit>,
}

impl HasCountSelectableObject for SelectableUnit {
    fn counter(&self) -> &CountSelector {
        &self.selectable
    }
}

impl RecruitAmount {
    pub fn new_selection(
        game: &Game,
        player_index: usize,
        city_position: Position,
        units: Units,
    ) -> StateUpdate {
        let player = game.player(player_index);
        let selectable: Vec<SelectableUnit> = new_units(player)
            .into_iter()
            .map(|u| selectable_unit(city_position, &units, player, u, game))
            .collect();

        StateUpdate::OpenDialog(ActiveDialog::RecruitUnitSelection(RecruitAmount {
            player_index,
            city_position,
            units,
            selectable,
        }))
    }
}

fn selectable_unit(
    city_position: Position,
    units: &Units,
    player: &Player,
    unit_type: UnitType,
    game: &Game,
) -> SelectableUnit {
    let mut all = units.clone();
    all += &unit_type;

    let current: u8 = units.get(&unit_type);

    let cost = recruit_cost_without_replaced(
        game,
        player,
        &all,
        city_position,
        CostTrigger::WithModifiers,
    );
    let max = if cost.is_ok() { current + 1 } else { current };
    SelectableUnit {
        unit_type,
        cost,
        selectable: CountSelector {
            current,
            min: 0,
            max,
        },
    }
}

fn new_units(player: &Player) -> Vec<UnitType> {
    vec![Settler, Infantry, Ship, Cavalry, Elephant]
        .into_iter()
        .chain(
            player
                .available_leaders
                .iter()
                .map(|l| Leader(*l))
                .collect_vec(),
        )
        .collect()
}

#[derive(Clone)]
pub struct RecruitSelection {
    pub player: usize,
    pub amount: RecruitAmount,
    pub available_units: Units,
    pub need_replacement: Units,
    pub replaced_units: Vec<u32>,
}

impl RecruitSelection {
    pub fn new(
        game: &Game,
        player: usize,
        amount: RecruitAmount,
        replaced_units: Vec<u32>,
    ) -> RecruitSelection {
        let available_units = game.player(amount.player_index).available_units();
        let need_replacement = get_units_to_replace(&available_units, &amount.units);

        RecruitSelection {
            player,
            amount,
            available_units,
            need_replacement,
            replaced_units,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.need_replacement.is_empty()
    }

    pub fn confirm(&self, game: &Game) -> OkTooltip {
        recruit_cost(
            game,
            game.player(self.amount.player_index),
            &self.amount.units,
            self.amount.city_position,
            self.replaced_units.as_slice(),
            CostTrigger::WithModifiers,
        )
        .map_or_else(
            |_| OkTooltip::Invalid("Replace exact amount of units".to_string()),
            |_| OkTooltip::Valid("Recruit units".to_string()),
        )
    }
}

impl UnitSelection for RecruitSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.replaced_units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.need_replacement.has_unit(&unit.unit_type)
    }

    fn player_index(&self) -> usize {
        self.player
    }
}

pub fn select_dialog(rc: &RenderContext, a: &RecruitAmount) -> StateUpdate {
    let game = rc.game;
    let radius = 20.;
    select_ui::count_dialog(
        rc,
        a,
        |s| s.selectable.clone(),
        |s, p| {
            draw_unit_type(
                rc,
                if s.cost.is_ok() {
                    HighlightType::None
                } else {
                    HighlightType::Warn
                },
                p,
                s.unit_type,
                rc.shown_player.index,
                radius,
            );
        },
        |s, p| {
            let suffix = match &s.cost {
                Ok(_) => format!(" ({} available with current resources)", s.selectable.max),
                Err(e) => format!(" ({e})"),
            };
            let mut tooltip = vec![format!("Recruit {}{}", s.unit_type.name(game), suffix)];
            add_unit_description(&mut tooltip, s.unit_type);
            show_tooltip_for_circle(rc, &tooltip, p, radius);
        },
        || OkTooltip::Valid("Recruit units".to_string()),
        || {
            let sel = RecruitSelection::new(game, rc.shown_player.index, a.clone(), vec![]);

            if sel.is_finished() {
                open_dialog(rc, a.city_position, sel)
            } else {
                StateUpdate::OpenDialog(ActiveDialog::ReplaceUnits(sel))
            }
        },
        |_s, _u| true,
        |s, u| {
            let mut units = s.units.clone();
            units.push(u.unit_type);
            update_selection(game, s, units)
        },
        |s, u| {
            let mut units = s.units.clone();
            remove_element(&mut units, &u.unit_type);
            update_selection(game, s, units)
        },
        Vec2::new(0., 0.),
        true,
    )
}

fn open_dialog(rc: &RenderContext, city: Position, sel: RecruitSelection) -> StateUpdate {
    let p = rc.shown_player.index;
    let cost = recruit_cost(
        rc.game,
        rc.shown_player,
        &sel.amount.units,
        city,
        &sel.replaced_units,
        CostTrigger::WithModifiers,
    )
    .unwrap();
    StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(ConstructionPayment::new(
        rc,
        rc.game.city(p, city),
        &format!(
            "Recruit {} in {city}",
            sel.amount.units.to_string(sel.amount.leader_name.as_ref()),
        ),
        ConstructionProject::Units(sel),
        &cost,
    )))
}

fn update_selection(game: &Game, s: &RecruitAmount, units: Vec<UnitType>) -> StateUpdate {
    RecruitAmount::new_selection(game, s.player_index, s.city_position, units, leader_name)
}

pub fn replace_dialog(rc: &RenderContext, sel: &RecruitSelection) -> StateUpdate {
    if ok_button(rc, sel.confirm(rc.game)) {
        open_dialog(rc, sel.amount.city_position, sel.clone())
    } else if cancel_button(rc) {
        StateUpdate::Cancel
    } else {
        StateUpdate::None
    }
}
