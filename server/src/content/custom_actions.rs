use serde::{Deserialize, Serialize};

use crate::content::wonders::construct_wonder;
use crate::log::{format_collect_log_item, format_happiness_increase};
use crate::player::Player;
use crate::playing_actions::{
    collect, increase_happiness, undo_collect, undo_increase_happiness, Collect, IncreaseHappiness,
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
    ForcedLabor,
    VotingIncreaseHappiness(IncreaseHappiness),
    FreeEconomyProduction(Collect),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub enum CustomActionType {
    ConstructWonder,
    ForcedLabor,
    VotingIncreaseHappiness,
    FreeEconomyCollect,
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
            CustomAction::ForcedLabor => {
                game.actions_left += 1;
            }
            CustomAction::VotingIncreaseHappiness(i) => {
                increase_happiness(game, player_index, i);
            }
            CustomAction::FreeEconomyProduction(c) => {
                collect(game, player_index, &c);
            }
        }
    }

    #[must_use]
    pub fn custom_action_type(&self) -> CustomActionType {
        match self {
            CustomAction::ConstructWonder { .. } => CustomActionType::ConstructWonder,
            CustomAction::ForcedLabor => CustomActionType::ForcedLabor,
            CustomAction::VotingIncreaseHappiness(_) => CustomActionType::VotingIncreaseHappiness,
            CustomAction::FreeEconomyProduction(_) => CustomActionType::FreeEconomyCollect,
        }
    }

    pub fn undo(self, game: &mut Game, player_index: usize) {
        let action = self.custom_action_type();
        if action.action_type().once_per_turn {
            game.played_once_per_turn_actions.retain(|a| a != &action);
        }
        match self {
            CustomAction::ConstructWonder {
                city_position,
                wonder: _,
                payment,
            } => {
                game.players[player_index].gain_resources(payment);
                let wonder = game.undo_build_wonder(city_position, player_index);
                game.players[player_index].wonder_cards.push(wonder);
            }
            CustomAction::ForcedLabor => game.actions_left -= 1,
            CustomAction::VotingIncreaseHappiness(i) => {
                undo_increase_happiness(game, player_index, i);
            }
            CustomAction::FreeEconomyProduction(c) => undo_collect(game, player_index, c),
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, player: &Player, player_name: &str) -> String {
        match self {
            CustomAction::ConstructWonder { city_position, wonder, payment } => format!("{player_name} paid {payment} to construct the {wonder} wonder in the city at {city_position}"),
            CustomAction::ForcedLabor => format!("{player_name} paid 2 mood tokens to get an extra action using Forced Labor"),
            CustomAction::VotingIncreaseHappiness(i) => format!("{} using Voting", format_happiness_increase(
                player,
                player_name,i
            )),
            CustomAction::FreeEconomyProduction(c) => format!("{} using Free Economy", format_collect_log_item(
                            player,
                            player_name,c
                        )),
        }
    }
}

impl CustomActionType {
    #[must_use]
    pub fn action_type(&self) -> ActionType {
        match self {
            CustomActionType::ConstructWonder => ActionType::default(),
            CustomActionType::ForcedLabor => {
                ActionType::free_and_once_per_turn(ResourcePile::mood_tokens(2))
            }
            CustomActionType::VotingIncreaseHappiness => {
                ActionType::free(ResourcePile::mood_tokens(1))
            }
            CustomActionType::FreeEconomyCollect => {
                ActionType::free_and_once_per_turn(ResourcePile::mood_tokens(1))
            }
        }
    }
}
