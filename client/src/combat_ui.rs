use crate::client_state::{ActiveDialog, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::payment_ui::{payment_model_dialog, PaymentModelPayment};
use crate::render_context::RenderContext;
use crate::select_ui::ConfirmSelection;
use crate::unit_ui;
use crate::unit_ui::UnitSelection;
use server::action::{Action, CombatAction, PlayActionCard};
use server::combat::CombatModifier;
use server::content::custom_phase_actions::{
    CustomPhaseAction, SiegecraftPayment, SIEGECRAFT_EXTRA_DIE, SIEGECRAFT_IGNORE_HIT,
};
use server::game::Game;
use server::payment::{get_single_resource_payment_model, PaymentModel};
use server::position::Position;
use server::resource_pile::ResourcePile;
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
    modifier: CombatModifier,
    model: PaymentModel,
    combat_value: ResourcePile,
}

impl SiegecraftPaymentModel {
    pub fn new(
        game: &Game,
        modifier: CombatModifier,
        combat_value: ResourcePile,
    ) -> SiegecraftPaymentModel {
        let cost = match modifier {
            CombatModifier::CancelFortressExtraDie => SIEGECRAFT_EXTRA_DIE,
            CombatModifier::CancelFortressIgnoreHit => SIEGECRAFT_IGNORE_HIT,
        };
        let mut available = game.get_player(game.active_player()).resources.clone();
        available -= combat_value.clone();

        let model = get_single_resource_payment_model(&available, &cost);
        SiegecraftPaymentModel {
            modifier,
            model,
            combat_value,
        }
    }

    pub fn next_modifier(game: &Game, last: Option<ResourcePile>) -> Option<CombatModifier> {
        let player = game.get_player(game.active_player());
        let resources = &player.resources;
        if let Some(extra_die) = last {
            if resources.can_afford(&(SIEGECRAFT_IGNORE_HIT + extra_die)) {
                return Some(CombatModifier::CancelFortressIgnoreHit);
            }
        } else {
            if resources.can_afford(&SIEGECRAFT_EXTRA_DIE) {
                return Some(CombatModifier::CancelFortressExtraDie);
            }
            if resources.can_afford(&SIEGECRAFT_IGNORE_HIT) {
                return Some(CombatModifier::CancelFortressIgnoreHit);
            }
        }
        None
    }
}

impl PaymentModelPayment for SiegecraftPaymentModel {
    fn payment_model(&self) -> &PaymentModel {
        &self.model
    }

    fn name(&self) -> &str {
        match self.modifier {
            CombatModifier::CancelFortressExtraDie => {
                "Cancel fortress extra die in first round of combat"
            }
            CombatModifier::CancelFortressIgnoreHit => {
                "Cancel fortress ignore hit in first round of combat"
            }
        }
    }

    fn new_dialog(&self) -> ActiveDialog {
        ActiveDialog::SiegecraftPayment(self.clone())
    }
}

pub fn pay_siegecraft_dialog(p: &SiegecraftPaymentModel, rc: &RenderContext) -> StateUpdate {
    payment_model_dialog(p, rc, |pile| {
        SiegecraftPaymentModel::next_modifier(rc.game, Some(pile.clone())).map_or_else(
            || {
                let mut extra_die = p.combat_value.clone();
                let mut ignore_hit = ResourcePile::empty();

                match p.modifier {
                    CombatModifier::CancelFortressExtraDie => {
                        extra_die += pile.clone();
                    }
                    CombatModifier::CancelFortressIgnoreHit => {
                        ignore_hit = pile.clone();
                    }
                }

                StateUpdate::Execute(Action::CustomPhase(
                    CustomPhaseAction::SiegecraftPaymentAction(SiegecraftPayment {
                        extra_die,
                        ignore_hit,
                    }),
                ))
            },
            |m| {
                StateUpdate::OpenDialog(ActiveDialog::SiegecraftPayment(
                    SiegecraftPaymentModel::new(rc.game, m, pile.clone()),
                ))
            },
        )
    })
}
