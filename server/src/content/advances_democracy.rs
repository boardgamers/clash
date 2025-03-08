use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city::MoodState;
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType::{
    CivilLiberties, FreeEconomyCollect, VotingIncreaseHappiness,
};
use crate::playing_actions::PlayingActionType;

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
    .add_player_event_listener(
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
    Advance::builder("Free Economy", "As a free action, you may spend 1 mood token to collect resources in one city. This must be your only collect action this turn")
        .add_custom_action(FreeEconomyCollect)
        .add_player_event_listener(
            |event| &mut event.is_playing_action_available,
            0,
            |available, game, i| {
                let p = game.get_player(i.player);
                if matches!(i.action_type, PlayingActionType::Collect) && p.played_once_per_turn_actions.contains(&FreeEconomyCollect) {
                    *available = false;
                }
            },
        )
}
