use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, gain_advance_without_payment};
use crate::civilization::Civilization;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::special_advance::{SpecialAdvance, SpecialAdvanceInfo};

pub(crate) fn rome() -> Civilization {
    Civilization::new(
        "Rome",
        vec![
            // todo sanitation cost 0 resources or free action
            SpecialAdvanceInfo::builder(
                SpecialAdvance::Aqueduct,
                Advance::Engineering,
                "Aqueduct",
                "Ignore Famine events. \
                Sanitation cost is reduced to 0 resources or a free action",
            )
            .add_custom_action(CustomActionType::Aqueduct)
            .build(),
        ],
        vec![],
    )
}

pub(crate) fn use_aqueduct() -> Builtin {
    Builtin::builder("Gain Aqueduct as a free action", "")
        .add_simple_persistent_event_listener(
            |event| &mut event.custom_action,
            0,
            |game, player, _name, a| {
                gain_advance_without_payment(
                    game,
                    Advance::Sanitation,
                    player,
                    a.payment.clone(),
                    true,
                );
            },
        )
        .build()
}
