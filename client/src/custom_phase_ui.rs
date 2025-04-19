use crate::advance_ui::{AdvanceState, show_advance_menu};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{BaseOrCustomDialog, OkTooltip, cancel_button_with_tooltip, ok_button};
use crate::layout_ui::{bottom_center_anchor, bottom_centered_text, icon_pos};
use crate::payment_ui::{Payment, multi_payment_dialog, payment_dialog};
use crate::player_ui::choose_player_dialog;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::tooltip::show_tooltip_for_circle;
use crate::unit_ui::{UnitSelection, add_unit_description, draw_unit_type};
use itertools::Itertools;
use macroquad::math::vec2;
use server::action::Action;
use server::content::persistent_events::{
    AdvanceRequest, EventResponse, MultiRequest, PlayerRequest, SelectedStructure, Structure,
    UnitTypeRequest, UnitsRequest, is_selected_structures_valid,
};
use server::cultural_influence::InfluenceCultureAttempt;
use server::game::Game;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::unit::Unit;

pub fn custom_phase_payment_dialog(rc: &RenderContext, payments: &[Payment]) -> StateUpdate {
    multi_payment_dialog(
        rc,
        payments,
        |p| ActiveDialog::PaymentRequest(p.clone()),
        false,
        |p| StateUpdate::Execute(Action::Response(EventResponse::Payment(p.clone()))),
    )
}

pub fn payment_reward_dialog(rc: &RenderContext, payment: &Payment) -> StateUpdate {
    payment_dialog(
        rc,
        payment,
        false,
        |p| ActiveDialog::ResourceRewardRequest(p.clone()),
        |p| StateUpdate::Execute(Action::Response(EventResponse::ResourceReward(p))),
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
                Action::Response(EventResponse::SelectAdvance(a.name.clone())),
            )
        },
    )
}

pub fn unit_request_dialog(rc: &RenderContext, r: &UnitTypeRequest) -> StateUpdate {
    bottom_centered_text(rc, &r.description);

    let c = &r.choices;
    let anchor = bottom_center_anchor(rc) + vec2(0., 60.);
    for pass in 0..2 {
        for (i, u) in c.iter().enumerate() {
            let x = (c.len() - i) as i8 - 1;
            let center = icon_pos(x, -2) + anchor;

            if pass == 0 {
                if draw_unit_type(rc, HighlightType::None, center, *u, r.player_index, 20.) {
                    return StateUpdate::Execute(Action::Response(EventResponse::SelectUnitType(
                        *u,
                    )));
                }
            } else {
                let mut tooltip = vec![u.name().to_string()];
                add_unit_description(&mut tooltip, *u);
                show_tooltip_for_circle(rc, &tooltip, center, 20.);
            }
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
        StateUpdate::response(EventResponse::SelectUnits(s.selection.selected.clone()))
    } else {
        StateUpdate::None
    }
}

#[derive(Clone)]
pub struct MultiSelection<T>
where
    T: Clone + PartialEq + Ord,
{
    pub request: MultiRequest<T>,
    pub selected: Vec<T>,
}

impl<T: Clone + PartialEq + Ord> MultiSelection<T> {
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
        }
        self
    }
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum SelectedStructureStatus {
    Valid,
    Warn,
    Invalid,
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct SelectedStructureInfo {
    pub position: Position,
    pub structure: Structure,
    pub status: SelectedStructureStatus,
    pub tooltip: Option<String>,
}

impl SelectedStructureInfo {
    pub fn new(
        position: Position,
        structure: Structure,
        status: SelectedStructureStatus,
        tooltip: Option<String>,
    ) -> Self {
        SelectedStructureInfo {
            position,
            structure,
            status,
            tooltip,
        }
    }

    pub fn selected(&self) -> SelectedStructure {
        SelectedStructure::new(self.position, self.structure.clone())
    }

    pub fn highlight_type(&self) -> HighlightType {
        match self.status {
            SelectedStructureStatus::Valid => HighlightType::Choices,
            SelectedStructureStatus::Warn => HighlightType::Warn,
            SelectedStructureStatus::Invalid => HighlightType::Invalid,
        }
    }
}

pub fn select_structures_dialog(
    rc: &RenderContext,
    d: Option<&BaseOrCustomDialog>,
    s: &MultiSelection<SelectedStructureInfo>,
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

    let sel = s
        .selected
        .iter()
        .map(SelectedStructureInfo::selected)
        .collect_vec();
    if ok_button(
        rc,
        multi_select_tooltip(
            s,
            s.request.is_valid(&s.selected) && is_selected_structures_valid(rc.game, &sel),
            "structures (city center must be the last one)",
        ),
    ) {
        if let Some(d) = d {
            if s.selected.is_empty() {
                return StateUpdate::CloseDialog;
            }
            StateUpdate::execute(Action::Playing(PlayingAction::InfluenceCultureAttempt(
                InfluenceCultureAttempt::new(s.selected[0].selected(), d.action_type.clone()),
            )))
        } else {
            StateUpdate::response(EventResponse::SelectStructures(sel))
        }
    } else {
        StateUpdate::None
    }
}

pub(crate) fn multi_select_tooltip<T: Clone + PartialEq + Ord>(
    s: &MultiSelection<T>,
    valid: bool,
    name: &str,
) -> OkTooltip {
    if valid {
        OkTooltip::Valid(format!("Select {name}"))
    } else {
        OkTooltip::Invalid(format!(
            "Need to select {} to {} {name}",
            s.request.needed.start(),
            s.request.needed.end()
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
    StateUpdate::Execute(Action::Response(EventResponse::Bool(answer)))
}

pub fn player_request_dialog(rc: &RenderContext, r: &PlayerRequest) -> StateUpdate {
    choose_player_dialog(rc, &r.choices, |p| {
        Action::Response(EventResponse::SelectPlayer(p))
    })
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
        StateUpdate::response(EventResponse::SelectPositions(s.selected.clone()))
    } else {
        StateUpdate::None
    }
}
