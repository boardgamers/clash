use crate::action::Action;
use crate::city::MoodState;
use crate::content::custom_actions::{CustomAction, CustomActionType};
use crate::content::persistent_events::SelectedStructure;
use crate::game::Game;
use crate::playing_actions::{Collect, IncreaseHappiness, PlayingAction, PlayingActionType};
use crate::position::Position;
use itertools::{Either, Itertools};

#[must_use]
pub fn base_and_custom_action(
    actions: Vec<PlayingActionType>,
) -> (Option<PlayingActionType>, Option<CustomActionType>) {
    let (mut custom, mut action): (Vec<_>, Vec<_>) = actions.into_iter().partition_map(|a| {
        if let PlayingActionType::Custom(c) = a {
            Either::Left(c.custom_action_type.clone())
        } else {
            Either::Right(a.clone())
        }
    });
    (action.pop(), custom.pop())
}

#[must_use]
pub fn available_collect_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    if game.player(player).get_city(position).can_activate() {
        available_collect_actions(game, player)
    } else {
        vec![]
    }
}

#[must_use]
pub fn available_collect_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::Collect,
        &CustomActionType::FreeEconomyCollect,
    )
}

///
/// # Panics
///
/// If the action is illegal
#[must_use]
pub fn collect_action(action: &PlayingActionType, collect: Collect) -> Action {
    match action {
        PlayingActionType::Collect => Action::Playing(PlayingAction::Collect(collect)),
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::FreeEconomyCollect =>
        {
            Action::Playing(PlayingAction::Custom(CustomAction::FreeEconomyCollect(
                collect,
            )))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
pub fn available_happiness_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    let city = game.player(player).get_city(position);
    if city.can_activate() && city.mood_state != MoodState::Happy {
        available_happiness_actions(game, player)
    } else {
        vec![]
    }
}

#[must_use]
pub fn available_happiness_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::IncreaseHappiness,
        &CustomActionType::VotingIncreaseHappiness,
    )
}

///
/// # Panics
///
/// If the action is illegal
#[must_use]
pub fn happiness_action(
    action: &PlayingActionType,
    include_happiness: IncreaseHappiness,
) -> Action {
    match action {
        PlayingActionType::IncreaseHappiness => {
            Action::Playing(PlayingAction::IncreaseHappiness(include_happiness))
        }
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::VotingIncreaseHappiness =>
        {
            Action::Playing(PlayingAction::Custom(
                CustomAction::VotingIncreaseHappiness(include_happiness),
            ))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
pub fn available_influence_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::InfluenceCultureAttempt,
        &CustomActionType::ArtsInfluenceCultureAttempt,
    )
}

///
/// # Panics
///
/// If the action is illegal
#[must_use]
pub fn influence_action(action: &PlayingActionType, target: SelectedStructure) -> Action {
    match action {
        PlayingActionType::InfluenceCultureAttempt => {
            Action::Playing(PlayingAction::InfluenceCultureAttempt(target))
        }
        PlayingActionType::Custom(c)
            if c.custom_action_type == CustomActionType::ArtsInfluenceCultureAttempt =>
        {
            Action::Playing(PlayingAction::Custom(
                CustomAction::ArtsInfluenceCultureAttempt(target),
            ))
        }
        _ => panic!("illegal type {action:?}"),
    }
}

#[must_use]
fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: &CustomActionType,
) -> Vec<PlayingActionType> {
    vec![action, custom.playing_action()]
        .into_iter()
        .filter_map(|a| a.is_available(game, player).map(|()| a).ok())
        .collect()
}
