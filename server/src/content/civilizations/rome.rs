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
            SpecialAdvanceInfo::builder(
                SpecialAdvance::Aqueduct,
                Advance::Engineering,
                "Aqueduct",
                "Ignore Famine events. \
                Sanitation cost is reduced to 0 resources or a free action",
            )
                .add_custom_action(CustomActionType::Aqueduct)
                .add_transient_event_listener(
                    |event| &mut event.advance_cost,
                    3,
                    |i, &a, _| {
                        if a == Advance::Sanitation {
                            i.set_zero();
                            i.info
                                .log
                                .push("Aqueduct reduced the cost to 0".to_string());
                        }
                    },
                )
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
            |game, player, name, a| {
                game.add_info_log_item(
                    &format!(
                        "{name} uses Aqueduct to gain Sanitation as a free action",
                    ),
                );
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
