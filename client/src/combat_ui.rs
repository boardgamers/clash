use macroquad::math::vec2;
use server::action::{Action, CombatAction, PlayActionCard};
use server::game::Game;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource::ResourceType;
use server::unit::Unit;
use crate::advance_ui::{show_paid_advance_menu, AdvancePayment};
use crate::client_state::StateUpdate;
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::layout_ui::bottom_center_text;
use crate::payment_ui::{payment_dialog, HasPayment, Payment};
use crate::render_context::RenderContext;
use crate::select_ui::ConfirmSelection;
use crate::unit_ui;
use crate::unit_ui::UnitSelection;

pub fn retreat_dialog(rc: &RenderContext) -> StateUpdate {
    if ok_button(rc, OkTooltip::Valid("Retreat".to_string())) {
        return retreat(true);
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return retreat(false);
    }
    StateUpdate::None
}

fn retreat(retreat: bool) -> StateUpdate {
    StateUpdate::Execute(Action::Combat(CombatAction::Retreat(retreat)))
}

#[derive(Clone)]
pub struct RemoveCasualtiesSelection {
    pub position: Position,
    pub needed: u8,
    pub selectable: Vec<u32>,
    pub units: Vec<u32>,
}

impl RemoveCasualtiesSelection {
    pub fn new(position: Position, needed: u8, selectable: Vec<u32>) -> Self {
        RemoveCasualtiesSelection {
            position,
            needed,
            units: Vec::new(),
            selectable,
        }
    }
}

impl UnitSelection for RemoveCasualtiesSelection {
    fn selected_units_mut(&mut self) -> &mut Vec<u32> {
        &mut self.units
    }

    fn can_select(&self, _game: &Game, unit: &Unit) -> bool {
        self.selectable.contains(&unit.id)
    }
}

impl ConfirmSelection for RemoveCasualtiesSelection {
    fn cancel_name(&self) -> Option<&str> {
        None
    }

    fn confirm(&self, _game: &Game) -> OkTooltip {
        if self.needed == self.units.len() as u8 {
            OkTooltip::Valid("Remove casualties".to_string())
        } else {
            OkTooltip::Invalid(format!(
                "Need to select {} units",
                self.needed - self.units.len() as u8
            ))
        }
    }
}

pub fn remove_casualties_dialog(
    rc: &RenderContext,
    sel: &RemoveCasualtiesSelection,
) -> StateUpdate {
    unit_ui::unit_selection_dialog::<RemoveCasualtiesSelection>(
        rc,
        sel,
        |new: RemoveCasualtiesSelection| {
            StateUpdate::Execute(Action::Combat(CombatAction::RemoveCasualties(
                new.units.clone(),
            )))
        },
    )
}

pub fn play_action_card_dialog(rc: &RenderContext) -> StateUpdate {
    if cancel_button_with_tooltip(rc, "Play no action card") {
        return StateUpdate::Execute(Action::Combat(CombatAction::PlayActionCard(
            PlayActionCard::None,
        )));
    }
    StateUpdate::None
}

#[derive(Clone)]
pub struct SiegecraftPayment {
    payment: Payment,
}

impl SiegecraftPayment {
    pub fn new(game: &Game, player_index: usize) -> SiegecraftPayment {
        let payment = game
            .get_player(player_index)
            .get_siegecraft_payment_options()
            .to_payment();
        SiegecraftPayment { payment }
    }
    
    pub fn valid(&self) -> OkTooltip {
        let pile = self.payment.to_resource_pile();
        let mut gold_max = 4;
        if pile.get(ResourceType::Wood) == 1 {
            gold_max -= 1;
        }
        if pile.get(ResourceType::Ore) == 1 {
            gold_max -= 1;
        }
        if self.payment.selectable.max > 0 {
            OkTooltip::Valid(format!("Pay {} to research siegecraft", self.payment))
        } else {
            OkTooltip::Invalid("Gold can be used as a replacement".to_string())
        }
    }
}

impl HasPayment for SiegecraftPayment {
    fn payment(&self) -> &Payment {
        &self.payment
    }
}

pub fn pay_siegecraft_dialog(p: &SiegecraftPayment, rc: &RenderContext) -> StateUpdate {
    bottom_center_text(rc, "Pay for siegecraft", vec2(-200., -50.));
    payment_dialog(
        p,
        AdvancePayment::valid,
        || {
            StateUpdate::Execute(Action::Playing(PlayingAction::Advance {
                advance: ap.name.to_string(),
                payment: ap.payment.to_resource_pile(),
            }))
        },
        |ap, r| ap.payment.get(r).selectable.max > 0,
        |ap, r| crate::advance_ui::add(ap, r, 1),
        |ap, r| crate::advance_ui::add(ap, r, -1),
        rc,
    )
}
