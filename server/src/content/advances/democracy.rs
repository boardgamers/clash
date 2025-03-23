use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city::MoodState;
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType::{
    CivilLiberties, FreeEconomyCollect, VotingIncreaseHappiness,
};
use crate::log::current_turn_log;
use crate::playing_actions::{PlayingAction, PlayingActionType};

pub(crate) fn democracy() -> AdvanceGroup {
    advance_group_builder(
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
    Advance::builder(
        "Voting",
        "As a free action, you may spend 1 mood token to gain an action 'Increase happiness'",
    )
    .add_custom_action(VotingIncreaseHappiness)
}

fn separation_of_power() -> AdvanceBuilder {
    Advance::builder(
        "Separation of Power",
        "Attempts to influence your happy cities may not be boosted by culture tokens",
    )
    .add_transient_event_listener(
        |event| &mut event.on_influence_culture_attempt,
        2,
        |info, city, _| {
            if matches!(city.mood_state, MoodState::Happy) {
                info.set_no_boost();
            }
        },
    )
}

fn civil_liberties() -> AdvanceBuilder {
    Advance::builder(
        "Civil Liberties",
        "As a free action, you may gain 3 mood tokens. The cost of Draft is increased to 2 mood token",
    )
        .add_custom_action(CivilLiberties)
}

fn free_economy() -> AdvanceBuilder {
    Advance::builder(
        "Free Economy",
        "As a free action, you may spend 1 mood token to collect \
                  resources in one city. This must be your only collect action this turn",
    )
    .add_custom_action(FreeEconomyCollect)
    .add_transient_event_listener(
        |event| &mut event.is_playing_action_available,
        0,
        |available, game, i| {
            let p = game.get_player(i.player);
            match &i.action_type {
                PlayingActionType::Collect
                    if p.played_once_per_turn_actions.contains(&FreeEconomyCollect) =>
                {
                    *available = Err("Cannot collect when Free Economy Collect was used".to_string());
                }
                PlayingActionType::Custom(i)
                    if matches!(i.custom_action_type, FreeEconomyCollect)
                        && current_turn_log(game).iter().any(|item| {
                            matches!(item.action, Action::Playing(PlayingAction::Collect(_)))
                        }) =>
                {
                    *available = Err("Cannot use Free Economy Collect when Collect was used".to_string());
                }
                _ => {}
            }
        },
    )
}
