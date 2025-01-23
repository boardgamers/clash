use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::payment_ui::{payment_model_dialog, PaymentModelEntry};
use crate::render_context::RenderContext;
use crate::select_ui::ConfirmSelection;
use crate::unit_ui;
use crate::unit_ui::UnitSelection;
use server::action::{Action, CombatAction, PlayActionCard};
use server::content::custom_phase_actions::{
    CustomPhaseAction, SiegecraftPayment, SIEGECRAFT_EXTRA_DIE, SIEGECRAFT_IGNORE_HIT,
};
use server::game::Game;
use server::payment::get_single_resource_payment_model;
use server::position::Position;
use server::unit::Unit;

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
pub struct SiegecraftPaymentModel {
    extra_die: PaymentModelEntry,
    ignore_hit: PaymentModelEntry,
}

impl SiegecraftPaymentModel {
    pub fn new(game: &Game) -> SiegecraftPaymentModel {
        // let cost = match modifier {
        //     CombatModifier::CancelFortressExtraDie => SIEGECRAFT_EXTRA_DIE,
        //     CombatModifier::CancelFortressIgnoreHit => SIEGECRAFT_IGNORE_HIT,
        // };
        let available = game.get_player(game.active_player()).resources.clone();
        // if let Some(extra_die) = &extra_die {
        //     available -= extra_die.clone();
        // }
        //
        // let model = get_single_resource_payment_model(&available, &cost);
        SiegecraftPaymentModel {
            extra_die: PaymentModelEntry {
                name: "Cancel fortress extra die in first round of combat".to_string(),
                model: get_single_resource_payment_model(&available, &SIEGECRAFT_EXTRA_DIE),
                optional: true,
            },
            ignore_hit: PaymentModelEntry {
                name: "Cancel fortress ignore hit in first round of combat".to_string(),
                model: get_single_resource_payment_model(&available, &SIEGECRAFT_IGNORE_HIT),
                optional: true,
            },
        }
    }
}

pub fn pay_siegecraft_dialog(p: &SiegecraftPaymentModel, rc: &RenderContext) -> StateUpdate {
    payment_model_dialog(
        rc,
        &vec![p.extra_die.clone(), p.ignore_hit.clone()],
        |p| {
            ActiveDialog::SiegecraftPayment(SiegecraftPaymentModel {
                extra_die: p[0].clone(),
                ignore_hit: p[1].clone(),
            })
        },
        |_| {
            StateUpdate::Execute(Action::CustomPhase(
                CustomPhaseAction::SiegecraftPaymentAction(SiegecraftPayment {
                    extra_die: p.extra_die.model.default().clone(),
                    ignore_hit: p.ignore_hit.model.default().clone(),
                }),
            ))
        },
    )
}
