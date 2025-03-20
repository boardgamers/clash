use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder};
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType::{AbsolutePower, ForcedLabor};
use crate::content::custom_phase_actions::ResourceRewardRequest;
use crate::payment::PaymentOptions;

pub(crate) fn autocracy() -> AdvanceGroup {
    advance_group_builder(
        "Autocracy",
        vec![
            nationalism(),
            totalitarianism(),
            absolute_power(),
            forced_labor(),
        ],
    )
}

fn nationalism() -> AdvanceBuilder {
    Advance::builder(
        "Nationalism",
        "Gain 1 mood or culture token when you recruit an army or ship unit.",
    )
    .add_resource_request(
        |event| &mut event.on_recruit,
        1,
        |_game, _player_index, recruit| {
            if recruit
                .units
                .clone()
                .to_vec()
                .iter()
                .any(|u| u.is_army_unit() || u.is_ship())
            {
                Some(ResourceRewardRequest::new(
                    PaymentOptions::tokens(1),
                    "Select token to gain".to_string(),
                ))
            } else {
                None
            }
        },
        |_game, resource, _| {
            vec![format!(
                "{} selected {} for Nationalism Advance",
                resource.player_name, resource.choice
            )]
        },
    )
}

fn totalitarianism() -> AdvanceBuilder {
    Advance::builder(
        "Totalitarianism",
        "Attempts to influence your cities with Army Units may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        0,
        |info, city, game| {
            if info.is_defender
                && game
                    .get_player(city.player_index)
                    .get_units(city.position)
                    .iter()
                    .any(|u| u.unit_type.is_army_unit())
            {
                info.set_no_boost();
            }
        },
    )
}

fn absolute_power() -> AdvanceBuilder {
    Advance::builder(
        "Absolute Power",
        "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action",
    )
    .add_custom_action(AbsolutePower)
}

fn forced_labor() -> AdvanceBuilder {
    Advance::builder(
        "Forced Labor",
        "Once per turn, as a free action, you may spend 1 mood token to treat your Angry cities as neutral for the rest of the turn",
    )
        .add_custom_action(ForcedLabor)
}
