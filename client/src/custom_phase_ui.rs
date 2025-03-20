use crate::advance_ui::{show_advance_menu, AdvanceState};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::layout_ui::{bottom_center_anchor, bottom_centered_text, icon_pos};
use crate::payment_ui::{multi_payment_dialog, payment_dialog, Payment};
use crate::player_ui::choose_player_dialog;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::unit_ui;
use crate::unit_ui::{draw_unit_type, UnitSelection};
use macroquad::math::vec2;
use server::action::Action;
use server::content::custom_phase_actions::{
    is_selected_structures_valid, AdvanceRequest, CurrentEventResponse, MultiRequest,
    PlayerRequest, SelectedStructure, Structure, UnitTypeRequest, UnitsRequest,
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
        |p| StateUpdate::Execute(Action::Response(CurrentEventResponse::Payment(p.clone()))),
    )
}

pub fn payment_reward_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::ResourceRewardRequest(p.clone()),
        |p| StateUpdate::Execute(Action::Response(CurrentEventResponse::ResourceReward(p))),
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
            } else if rc.shown_player.has_advance(&a.name) {
                AdvanceState::Owned
            } else {
                AdvanceState::Unavailable
            }
        },
        |a| {
            StateUpdate::execute_with_confirm(
                vec![format!("Select {}?", a.name)],
                Action::Response(CurrentEventResponse::SelectAdvance(a.name.clone())),
            )
        },
    )
}

pub fn unit_request_dialog(rc: &RenderContext, r: &UnitTypeRequest) -> StateUpdate {
    bottom_centered_text(rc, &r.description);

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
            return StateUpdate::Execute(Action::Response(CurrentEventResponse::SelectUnitType(
                *u,
            )));
        }
    }

    StateUpdate::None
}

#[derive(Clone)]
pub struct UnitsSelection {
    pub player: usize,
    pub selection: MultiSelection<u32>,
}

impl UnitsSelection {
    pub fn new(r: &UnitsRequest) -> Self {
        UnitsSelection {
            player: r.player,
            selection: MultiSelection::new(r.request.clone()),
        }
    }
}

impl UnitSelection for UnitsSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.selection.selected
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selection.request.choices.contains(&unit.id)
    }

    fn player_index(&self) -> usize {
        self.player
    }
}

pub fn select_units_dialog(rc: &RenderContext, s: &UnitsSelection) -> StateUpdate {
    bottom_centered_text(
        rc,
        format!(
            "{}: {} units selected",
            s.selection.request.description,
            s.selection.selected.len()
        )
        .as_str(),
    );

    if ok_button(
        rc,
        multi_select_tooltip(&s.selection, s.selection.is_valid(), "units"),
    ) {
        StateUpdate::response(CurrentEventResponse::SelectUnits(
            s.selection.selected.clone(),
        ))
    } else {
        StateUpdate::None
    }
}

#[derive(Clone)]
pub struct MultiSelection<T>
where
    T: Clone,
{
    pub request: MultiRequest<T>,
    pub selected: Vec<T>,
}

impl<T: Clone + PartialEq> MultiSelection<T> {
    pub fn new(request: MultiRequest<T>) -> Self {
        MultiSelection {
            request,
            selected: vec![],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.request.is_valid(&self.selected)
    }

    pub fn toggle(self, value: T) -> Self {
        if let Some(i) = self.selected.iter().position(|s| s == &value) {
            let mut new = self.clone();
            new.selected.remove(i);
            return new;
        }
        if self.request.choices.contains(&value) {
            let mut new = self.clone();
            new.selected.push(value);
            return new;
        };
        self
    }
}

pub fn select_structures_dialog(
    rc: &RenderContext,
    s: &MultiSelection<SelectedStructure>,
) -> StateUpdate {
    bottom_centered_text(
        rc,
        format!(
            "{}: {} structures selected",
            s.request.description,
            s.selected.len()
        )
        .as_str(),
    );

    if ok_button(
        rc,
        multi_select_tooltip(
            s,
            s.request.is_valid(&s.selected) && is_selected_structures_valid(rc.game, &s.selected),
            "structures (city center must be the last one)",
        ),
    ) {
        StateUpdate::response(CurrentEventResponse::SelectStructures(s.selected.clone()))
    } else {
        StateUpdate::None
    }
}

pub(crate) fn multi_select_tooltip<T: Clone>(
    s: &MultiSelection<T>,
    valid: bool,
    name: &str,
) -> OkTooltip {
    if valid {
        OkTooltip::Valid(format!("Select {name}"))
    } else {
        OkTooltip::Invalid(format!(
            "Need to select {} to {} {name}",
            s.request.needed.clone().min().unwrap(),
            s.request.needed.clone().max().unwrap()
        ))
    }
}

pub fn bool_request_dialog(rc: &RenderContext, description: &str) -> StateUpdate {
    bottom_centered_text(rc, description);
    if ok_button(rc, OkTooltip::Valid("OK".to_string())) {
        return bool_answer(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return bool_answer(false);
    }
    StateUpdate::None
}

fn bool_answer(answer: bool) -> StateUpdate {
    StateUpdate::Execute(Action::Response(CurrentEventResponse::Bool(answer)))
}

pub fn player_request_dialog(rc: &RenderContext, r: &PlayerRequest) -> StateUpdate {
    choose_player_dialog(rc, &r.choices, |p| {
        Action::Response(CurrentEventResponse::SelectPlayer(p))
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

pub(crate) fn position_request_dialog(
    rc: &RenderContext,
    s: &MultiSelection<Position>,
) -> StateUpdate {
    bottom_centered_text(
        rc,
        format!("{}: {} selected", s.request.description, s.selected.len()).as_str(),
    );
    if ok_button(rc, multi_select_tooltip(s, s.is_valid(), "positions")) {
        StateUpdate::response(CurrentEventResponse::SelectPositions(s.selected.clone()))
    } else {
        StateUpdate::None
    }
}
