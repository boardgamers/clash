use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType::{AbsolutePower, ForcedLabor};
use crate::content::persistent_events::ResourceRewardRequest;
use crate::payment::ResourceReward;

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
    AdvanceInfo::builder(
        Advance::Nationalism,
        "Nationalism",
        "Gain 1 mood or culture token when you recruit an army or ship unit.",
    )
    .add_resource_request(
        |event| &mut event.recruit,
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
                    ResourceReward::tokens(1),
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
    AdvanceInfo::builder(
        Advance::Totalitarianism,
        "Totalitarianism",
        "Attempts to influence your cities with Army Units may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        0,
        |r, city, game| {
            if let Ok(info) = r {
                if info.is_defender
                    && game
                        .player(city.player_index)
                        .get_units(city.position)
                        .iter()
                        .any(|u| u.unit_type.is_army_unit())
                {
                    info.set_no_boost();
                }
            }
        },
    )
}

const ABSOLUTE_POWER: &str =
    "Once per turn, as a free action, you may spend 2 mood tokens to get an additional action";

fn absolute_power() -> AdvanceBuilder {
    AdvanceInfo::builder(Advance::AbsolutePower, "Absolute Power", ABSOLUTE_POWER)
        .add_custom_action(AbsolutePower)
}

pub(crate) fn use_absolute_power() -> Builtin {
    Builtin::builder("Absolute Power", ABSOLUTE_POWER)
        .add_simple_persistent_event_listener(
            |event| &mut event.custom_action,
            0,
            |game, _, player_name, _| {
                game.actions_left += 1;
                game.add_info_log_item(&format!(
                    "{player_name} got an extra action using Absolute Power",
                ));
            },
        )
        .build()
}

const FORCED_LABOR: &str = "Once per turn, as a free action, \
    you may spend 1 mood token to treat your Angry cities as neutral for the rest of the turn";

fn forced_labor() -> AdvanceBuilder {
    AdvanceInfo::builder(Advance::ForcedLabor, "Forced Labor", FORCED_LABOR)
        .add_custom_action(ForcedLabor)
}

pub(crate) fn use_forced_labor() -> Builtin {
    Builtin::builder("Forced Labor", FORCED_LABOR)
        .add_simple_persistent_event_listener(
            |event| &mut event.custom_action,
            0,
            |game, _, player_name, _| {
                // we check that the action was played
                game.add_info_log_item(&format!(
                    "{player_name} paid 1 mood token to treat Angry cities as neutral"
                ));
            },
        )
        .build()
}
