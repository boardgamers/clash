use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::Action;
use crate::advance::{Advance, AdvanceBuilder};
use crate::city::MoodState;
use crate::content::advances::{AdvanceGroup, advance_group_builder};
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType::{
    CivilLiberties, FreeEconomyCollect, VotingIncreaseHappiness,
};
use crate::log::current_player_turn_log;
use crate::playing_actions::{PlayingAction, PlayingActionType};
use crate::resource_pile::ResourcePile;

pub(crate) fn democracy() -> AdvanceGroup {
    advance_group_builder("Democracy", vec![
        voting(),
        separation_of_power(),
        civil_liberties(),
        free_economy(),
    ])
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
        |r, city, _| {
            if let Ok(info) = r {
                if matches!(city.mood_state, MoodState::Happy) {
                    info.set_no_boost();
                }
            }
        },
    )
}

fn civil_liberties() -> AdvanceBuilder {
    Advance::builder(
        "Civil Liberties",
        "As a free action, you may gain 3 mood tokens. \
        The cost of Draft is increased to 2 mood token",
    )
    .add_custom_action(CivilLiberties)
}

pub(crate) fn use_civil_liberties() -> Builtin {
    Builtin::builder("Civil Liberties", "")
        .add_simple_persistent_event_listener(
            |event| &mut event.custom_action,
            0,
            |game, player_index, player_name, _| {
                game.player_mut(player_index)
                    .gain_resources(ResourcePile::mood_tokens(3));
                game.add_info_log_item(&format!(
                    "{player_name} gained 3 mood tokens using Civil Liberties"
                ));
            },
        )
        .build()
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
            let p = game.player(i.player);
            match &i.action_type {
                PlayingActionType::Collect
                    if p.played_once_per_turn_actions.contains(&FreeEconomyCollect) =>
                {
                    *available =
                        Err("Cannot collect when Free Economy Collect was used".to_string());
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
            }
        },
    )
}
