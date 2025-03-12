use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::collect::collect;
use crate::content::advances::culture::{execute_sports, execute_theaters};
use crate::content::advances::economy::collect_taxes;
use crate::content::wonders::construct_wonder;
use crate::cultural_influence::influence_culture_attempt;
use crate::log::{
    current_turn_log, format_city_happiness_increase, format_collect_log_item,
    format_cultural_influence_attempt_log_item, format_happiness_increase,
};
use crate::player::Player;
use crate::playing_actions::{
    increase_happiness, Collect, IncreaseHappiness, InfluenceCultureAttempt, PlayingAction,
};
use crate::{
    game::Game, playing_actions::ActionType, position::Position, resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum CustomAction {
    ConstructWonder {
        city_position: Position,
        wonder: String,
        payment: ResourcePile,
    },
    AbsolutePower,
    ForcedLabor,
    CivilRights,
    ArtsInfluenceCultureAttempt(InfluenceCultureAttempt),
    VotingIncreaseHappiness(IncreaseHappiness),
    FreeEconomyCollect(Collect),
    Sports {
        city_position: Position,
        payment: ResourcePile,
    },
    Taxes(ResourcePile),
    Theaters(ResourcePile),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CustomActionType {
    ConstructWonder,
    AbsolutePower,
    ForcedLabor,
    CivilLiberties,
    ArtsInfluenceCultureAttempt,
    VotingIncreaseHappiness,
    FreeEconomyCollect,
    Sports,
    Taxes,
    Theaters,
}

impl CustomAction {
    ///
    ///
    /// # Panics
    ///
    /// Panics if action is illegal
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder,
                payment,
            } => construct_wonder(game, player_index, city_position, &wonder, payment),
            CustomAction::AbsolutePower => game.actions_left += 1,
            CustomAction::ForcedLabor => {
                // we check that the action was played
            }
            CustomAction::CivilRights => {
                game.players[player_index].gain_resources(ResourcePile::mood_tokens(3));
            }
            CustomAction::ArtsInfluenceCultureAttempt(c) => {
                influence_culture_attempt(game, player_index, &c);
            }
            CustomAction::VotingIncreaseHappiness(i) => {
                increase_happiness(game, player_index, &i.happiness_increases, Some(i.payment));
            }
            CustomAction::FreeEconomyCollect(c) => collect(game, player_index, &c),
            CustomAction::Sports {
                city_position,
                payment,
            } => {
                execute_sports(game, player_index, city_position, &payment);
            }
            CustomAction::Taxes(r) => collect_taxes(game, player_index, r),
            CustomAction::Theaters(r) => execute_theaters(game, player_index, &r),
        }
    }

    #[must_use]
    pub fn custom_action_type(&self) -> CustomActionType {
        match self {
            CustomAction::ConstructWonder { .. } => CustomActionType::ConstructWonder,
            CustomAction::AbsolutePower => CustomActionType::AbsolutePower,
            CustomAction::ForcedLabor => CustomActionType::ForcedLabor,
            CustomAction::CivilRights => CustomActionType::CivilLiberties,
            CustomAction::ArtsInfluenceCultureAttempt(_) => {
                CustomActionType::ArtsInfluenceCultureAttempt
            }
            CustomAction::VotingIncreaseHappiness(_) => CustomActionType::VotingIncreaseHappiness,
            CustomAction::FreeEconomyCollect(_) => CustomActionType::FreeEconomyCollect,
            CustomAction::Sports { .. } => CustomActionType::Sports,
            CustomAction::Taxes(_) => CustomActionType::Taxes,
            CustomAction::Theaters(_) => CustomActionType::Theaters,
        }
    }

    #[must_use]
    pub fn format_log_item(&self, game: &Game, player: &Player, player_name: &str) -> String {
        match self {
            CustomAction::ConstructWonder { city_position, wonder, payment } =>
                format!("{player_name} paid {payment} to construct the {wonder} wonder in the city at {city_position}"),
            CustomAction::AbsolutePower =>
                format!("{player_name} paid 2 mood tokens to get an extra action using Forced Labor"),
            CustomAction::ForcedLabor =>
                format!("{player_name} paid 1 mood token to treat Angry cities as neutral"),
            CustomAction::CivilRights =>
                format!("{player_name} gained 3 mood tokens using Civil Liberties"),
            CustomAction::ArtsInfluenceCultureAttempt(c) =>
                format!("{} using Arts", format_cultural_influence_attempt_log_item(game, player.index, player_name, c)),
            CustomAction::VotingIncreaseHappiness(i) =>
                format!("{} using Voting", format_happiness_increase(player, player_name, i)),
            CustomAction::FreeEconomyCollect(c) =>
                format!("{} using Free Economy", format_collect_log_item(player, player_name, c)),
            CustomAction::Sports { city_position, payment } =>
                format!("{player_name} paid {payment} to increase the happiness in {} using Sports",
                    format_city_happiness_increase(player, *city_position, payment.amount())),
            CustomAction::Taxes(r) =>
                format!("{player_name} paid 1 mood token to collect {r} using Taxes"),
            CustomAction::Theaters(r) =>
                format!("{player_name} paid {r} to convert resources using Theaters"),
        }
    }
}

impl CustomActionType {
    #[must_use]
    pub fn action_type(&self) -> ActionType {
        match self {
            CustomActionType::AbsolutePower => {
                ActionType::free_and_once_per_turn(ResourcePile::mood_tokens(2))
            }
            CustomActionType::CivilLiberties
            | CustomActionType::Sports
            | CustomActionType::ConstructWonder => ActionType::default(),
            CustomActionType::ArtsInfluenceCultureAttempt => {
                ActionType::free_and_once_per_turn(ResourcePile::culture_tokens(1))
            }
            CustomActionType::VotingIncreaseHappiness => {
                ActionType::free(ResourcePile::mood_tokens(1))
            }
            CustomActionType::FreeEconomyCollect | CustomActionType::ForcedLabor => {
                ActionType::free_and_once_per_turn(ResourcePile::mood_tokens(1))
            }
            CustomActionType::Taxes => ActionType::once_per_turn(ResourcePile::mood_tokens(1)),
            CustomActionType::Theaters => ActionType::free_and_once_per_turn(ResourcePile::empty()),
        }
    }

    #[must_use]
    pub fn is_available(&self, game: &Game, _player_index: usize) -> bool {
        match self {
            CustomActionType::FreeEconomyCollect => !current_turn_log(game)
                .iter()
                .any(|item| matches!(item.action, Action::Playing(PlayingAction::Collect(_)))),
            _ => true,
        }
    }
}
