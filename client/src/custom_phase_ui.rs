use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::layout_ui::{bottom_center_anchor, icon_pos};
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::player_ui::choose_player_dialog;
use crate::render_context::RenderContext;
use crate::select_ui::{may_cancel, ConfirmSelection, HighlightType};
use crate::unit_ui;
use crate::unit_ui::{draw_unit_type, UnitSelection};
use macroquad::math::vec2;
use server::action::Action;
use server::content::custom_phase_actions::{
    AdvanceRequest, CurrentEventResponse, PlayerRequest, Structure, StructuresRequest,
    UnitTypeRequest,
};
use server::game::Game;
use server::position::Position;
use server::unit::Unit;

pub fn custom_phase_payment_dialog(rc: &RenderContext, payments: &[Payment]) -> StateUpdate {
    multi_payment_dialog(
        rc,
        payments,
        |p| ActiveDialog::PaymentRequest(p.clone()),
        false,
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(CurrentEventResponse::Payment(
                p.clone(),
            )))
        },
    )
}

pub fn payment_reward_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::ResourceRewardRequest(p.clone()),
        |p| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CurrentEventResponse::ResourceReward(p),
            ))
        },
    )
}

pub fn advance_reward_dialog(rc: &RenderContext, r: &AdvanceRequest, name: &str) -> StateUpdate {
    let possible = &r.choices;
    show_advance_menu(
        rc,
        &format!("Select advance for {name}"),
        |a, _| {
            if possible.contains(&a.name) {
                AdvanceState::Available
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            StateUpdate::Execute(Action::CustomPhaseEvent(
                CurrentEventResponse::SelectAdvance(a.name.clone()),
            ))
        },
    )
}

pub fn unit_request_dialog(rc: &RenderContext, r: &UnitTypeRequest) -> StateUpdate {
    let c = &r.choices;
    let anchor = bottom_center_anchor(rc) + vec2(0., 60.);
    for (i, u) in c.iter().enumerate() {
        let x = (c.len() - i) as i8 - 1;
        let p = icon_pos(x, -2) + anchor;

        if draw_unit_type(
            rc,
            HighlightType::None,
            p,
            *u,
            r.player_index,
            unit_ui::name(u),
            20.,
        ) {
            return StateUpdate::Execute(Action::CustomPhaseEvent(
                CurrentEventResponse::SelectUnitType(*u),
            ));
        }
    }

    StateUpdate::None
}

#[derive(Clone)]
pub struct UnitsSelection {
    pub needed: u8,
    pub player: usize,
    pub selectable: Vec<u32>,
    pub units: Vec<u32>,
    pub description: Option<String>,
}

impl UnitsSelection {
    pub fn new(
        player: usize,
        needed: u8,
        selectable: Vec<u32>,
        description: Option<String>,
    ) -> Self {
        UnitsSelection {
            player,
            needed,
            units: Vec::new(),
            selectable,
            description,
        }
    }
}

impl UnitSelection for UnitsSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selectable.contains(&unit.id)
    }

    fn player_index(&self) -> usize {
        self.player
    }
}

impl ConfirmSelection for UnitsSelection {
    fn cancel_name(&self) -> Option<&str> {
        None
    }

    fn confirm(&self, _game: &Game) -> OkTooltip {
        if self.units.len() as u8 == self.needed {
            OkTooltip::Valid("Select units".to_string())
        } else {
            OkTooltip::Invalid(format!(
                "Need to select {} units",
                self.needed as i8 - self.units.len() as i8
            ))
        }
    }
}

pub fn select_units_dialog(rc: &RenderContext, sel: &UnitsSelection) -> StateUpdate {
    unit_ui::unit_selection_dialog::<UnitsSelection>(rc, sel, |new: UnitsSelection| {
        StateUpdate::Execute(Action::CustomPhaseEvent(CurrentEventResponse::SelectUnits(
            new.units.clone(),
        )))
    })
}

#[derive(Clone)]
pub struct StructuresSelection {
    pub request: StructuresRequest,
    pub structures: Vec<(Position, Structure)>,
}

impl StructuresSelection {
    pub fn new(s: &StructuresRequest) -> Self {
        StructuresSelection {
            request: s.clone(),
            structures: Vec::new(),
        }
    }
}

impl ConfirmSelection for StructuresSelection {
    fn cancel_name(&self) -> Option<&str> {
        None
    }

    fn confirm(&self, game: &Game) -> OkTooltip {
        if self.request.is_valid(game, &self.structures) {
            OkTooltip::Valid("Select structures".to_string())
        } else {
            OkTooltip::Invalid(format!(
                "Need to select {} to {} structures (city center must be the last one)",
                self.request.needed.clone().min().unwrap(),
                self.request.needed.clone().max().unwrap()
            ))
        }
    }
}

pub fn select_structures_dialog(rc: &RenderContext, sel: &StructuresSelection) -> StateUpdate {
    if ok_button(rc, sel.confirm(rc.game)) {
        StateUpdate::Execute(Action::CustomPhaseEvent(
            CurrentEventResponse::SelectStructures(sel.structures.clone()),
        ))
    } else {
        may_cancel(sel, rc)
    }
}

pub fn bool_request_dialog(rc: &RenderContext) -> StateUpdate {
    if ok_button(rc, OkTooltip::Valid("OK".to_string())) {
        return bool_answer(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return bool_answer(false);
    }
    StateUpdate::None
}

fn bool_answer(answer: bool) -> StateUpdate {
    StateUpdate::Execute(Action::CustomPhaseEvent(CurrentEventResponse::Bool(answer)))
}

pub fn player_request_dialog(rc: &RenderContext, r: &PlayerRequest) -> StateUpdate {
    choose_player_dialog(rc, &r.choices, |p| {
        Action::CustomPhaseEvent(CurrentEventResponse::SelectPlayer(p))
    })
}

pub struct StructureHighlight {
    pub structure: Structure,
    pub highlight_type: HighlightType,
}

pub fn highlight_structures(
    structures: &[Structure],
    highlight_type: HighlightType,
) -> Vec<StructureHighlight> {
    structures
        .iter()
        .map(move |s| StructureHighlight {
            structure: s.clone(),
            highlight_type,
        })
        .collect()
}
