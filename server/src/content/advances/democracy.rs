use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::advance::{Advance, AdvanceBuilder, AdvanceInfo};
use crate::city::MoodState;
use crate::content::advances::{AdvanceGroup, AdvanceGroupInfo, advance_group_builder};
use crate::content::custom_actions::CustomActionType::{
    CivilLiberties, FreeEconomyCollect, VotingIncreaseHappiness,
};
use crate::log::current_player_turn_log;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::resource_pile::ResourcePile;

pub(crate) fn democracy() -> AdvanceGroupInfo {
    advance_group_builder(
        AdvanceGroup::Democracy,
        "Democracy",
        vec![
            voting(),
            separation_of_power(),
            civil_liberties(),
            free_economy(),
        ],
    )
}

fn voting() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::Voting,
        "Voting",
        "As a free action, you may spend 1 mood token to use 'Increase happiness'",
    )
    .add_action_modifier(
        VotingIncreaseHappiness,
        |c| {
            c.any_times()
                .free_action()
                .resources(ResourcePile::mood_tokens(1))
        },
        PlayingActionType::IncreaseHappiness,
    )
}

fn separation_of_power() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::SeparationOfPower,
        "Separation of Power",
        "Attempts to influence your happy cities may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        2,
        |r, city, _, _| {
            if let Ok(info) = r {
                if matches!(city.mood_state, MoodState::Happy) {
                    info.set_no_boost();
                }
            }
        },
    )
}

fn civil_liberties() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::CivilLiberties,
        "Civil Liberties",
        "As an action, you may gain 3 mood tokens. \
            The cost of Draft is increased to 2 mood token",
    )
    .add_custom_action(
        CivilLiberties,
        |c| c.any_times().action().no_resources(),
        |b| {
            b.add_simple_persistent_event_listener(
                |event| &mut event.custom_action,
                0,
                |game, p, _| {
                    p.gain_resources(game, ResourcePile::mood_tokens(3));
                },
            )
        },
        |_, _| true,
    )
}

fn free_economy() -> AdvanceBuilder {
    AdvanceInfo::builder(
        Advance::FreeEconomy,
        "Free Economy",
        "As a free action, you may spend 1 mood token to collect \
            resources in one city. This must be your only collect action this turn",
    )
    .add_action_modifier(
        FreeEconomyCollect,
        |c| {
            c.once_per_turn()
                .free_action()
                .resources(ResourcePile::mood_tokens(1))
        },
        PlayingActionType::Collect,
    )
    .add_transient_event_listener(
        |event| &mut event.is_playing_action_available,
        0,
        |available, game, t, p| match t {
            PlayingActionType::Collect
                if p.get(game)
                    .played_once_per_turn_actions
                    .contains(&FreeEconomyCollect) =>
            {
                *available = Err("Cannot collect when Free Economy Collect was used".to_string());
            }
            PlayingActionType::Custom(i)
                if *i == FreeEconomyCollect
                    && current_player_turn_log(game).items.iter().any(|item| {
                        matches!(&item.action, Action::Playing(PlayingAction::Collect(c)) if
                                c.action_type == PlayingActionType::Collect)
                    }) =>
            {
                *available =
                    Err("Cannot use Free Economy Collect when Collect was used".to_string());
            }
            _ => {}
        },
    )
}
