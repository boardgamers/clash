use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::{Advance, AdvanceBuilder};
use crate::content::advances::{advance_group_builder, AdvanceGroup};
use crate::content::custom_actions::CustomActionType::{
    CivilRights, FreeEconomyCollect, VotingIncreaseHappiness,
};
use crate::playing_actions::PlayingActionType;

pub(crate) fn democracy() -> AdvanceGroup {
    advance_group_builder("Democracy", vec![voting(), civil_rights(), free_economy()])
}

fn voting() -> AdvanceBuilder {
    Advance::builder(
        "Voting",
        "As a free action, you may spend 1 mood token to gain an action 'Increase happiness'",
    )
    .add_custom_action(VotingIncreaseHappiness)
}

fn civil_rights() -> AdvanceBuilder {
    Advance::builder(
        "Civil Rights",
        "As a free action, you may gain 3 mood tokens. The cost of Draft is increased to 2 mood token",
    )
        .add_custom_action(CivilRights)
}

fn free_economy() -> AdvanceBuilder {
    Advance::builder("Free Economy", "As a free action, you may spend 1 mood token to collect resources in one city. This must be your only collect action this turn")
        .add_custom_action(FreeEconomyCollect)
        .add_player_event_listener(
            |event| &mut event.is_playing_action_available,
            |available, action_type, player| {
                if matches!(action_type, PlayingActionType::Collect) && player.played_once_per_turn_actions.contains(&FreeEconomyCollect) {
                    *available = false;
                }
            },
            0,
        )
}
