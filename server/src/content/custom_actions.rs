use serde::{Deserialize, Serialize};

use crate::city::City;
use crate::collect::collect;
use crate::content::advances::culture::{sports_options, use_sports, use_theaters};
use crate::content::advances::economy::{collect_taxes, use_bartering};
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{PersistentEventType, SelectedStructure};
use crate::cultural_influence::{
    format_cultural_influence_attempt_log_item, influence_culture_attempt,
};
use crate::happiness::increase_happiness;
use crate::log::{format_collect_log_item, format_happiness_increase};
use crate::player::Player;
use crate::playing_actions::{Collect, IncreaseHappiness, PlayingActionType};
use crate::position::Position;
use crate::{game::Game, playing_actions::ActionCost, resource_pile::ResourcePile};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct CustomEventAction {
    pub action: CustomActionType,
    pub city: Option<Position>,
}

impl CustomEventAction {
    #[must_use]
    pub fn new(action: CustomActionType, city: Option<Position>) -> Self {
        Self { action, city }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum CustomAction {
    AbsolutePower,
    ForcedLabor,
    CivilLiberties,
    ArtsInfluenceCultureAttempt(SelectedStructure),
    VotingIncreaseHappiness(IncreaseHappiness),
    FreeEconomyCollect(Collect),
    Taxes(ResourcePile),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomActionInfo {
    pub custom_action_type: CustomActionType,
    pub action_type: ActionCost,
    pub once_per_turn: bool,
}

impl CustomActionInfo {
    #[must_use]
    fn new(
        custom_action_type: &CustomActionType,
        free: bool,
        once_per_turn: bool,
        cost: ResourcePile,
    ) -> CustomActionInfo {
        CustomActionInfo {
            custom_action_type: custom_action_type.clone(),
            action_type: ActionCost::new(free, cost),
            once_per_turn,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Debug)]
pub enum CustomActionType {
    AbsolutePower,
    ForcedLabor,
    CivilLiberties,
    Bartering,
    ArtsInfluenceCultureAttempt,
    VotingIncreaseHappiness,
    FreeEconomyCollect,
    Sports,
    Taxes,
    Theaters,
}

impl CustomAction {
    pub(crate) fn execute(self, game: &mut Game, player_index: usize) -> Result<(), String> {
        match self {
            CustomAction::AbsolutePower => game.actions_left += 1,
            CustomAction::ForcedLabor => {
                // we check that the action was played
            }
            CustomAction::CivilLiberties => {
                game.players[player_index].gain_resources(ResourcePile::mood_tokens(3));
            }
            CustomAction::ArtsInfluenceCultureAttempt(c) => {
                influence_culture_attempt(
                    game,
                    player_index,
                    &c,
                    &CustomActionType::ArtsInfluenceCultureAttempt.playing_action_type(),
                );
            }
            CustomAction::VotingIncreaseHappiness(i) => {
                increase_happiness(game, player_index, &i.happiness_increases, Some(i.payment));
            }
            CustomAction::FreeEconomyCollect(c) => collect(game, player_index, &c)?,
            CustomAction::Taxes(r) => collect_taxes(game, player_index, r),
        }
        Ok(())
    }

    #[must_use]
    pub fn custom_action_type(&self) -> CustomActionType {
        match self {
            CustomAction::AbsolutePower => CustomActionType::AbsolutePower,
            CustomAction::ForcedLabor => CustomActionType::ForcedLabor,
            CustomAction::CivilLiberties => CustomActionType::CivilLiberties,
            CustomAction::ArtsInfluenceCultureAttempt(_) => {
                CustomActionType::ArtsInfluenceCultureAttempt
            }
            CustomAction::VotingIncreaseHappiness(_) => CustomActionType::VotingIncreaseHappiness,
            CustomAction::FreeEconomyCollect(_) => CustomActionType::FreeEconomyCollect,
            CustomAction::Taxes(_) => CustomActionType::Taxes,
        }
    }

    #[must_use]
    pub fn format_log_item(&self, game: &Game, player: &Player, player_name: &str) -> String {
        match self {
            CustomAction::AbsolutePower => format!(
                "{player_name} paid 2 mood tokens to get an extra action using Forced Labor"
            ),
            CustomAction::ForcedLabor => {
                format!("{player_name} paid 1 mood token to treat Angry cities as neutral")
            }
            CustomAction::CivilLiberties => {
                format!("{player_name} gained 3 mood tokens using Civil Liberties")
            }
            CustomAction::ArtsInfluenceCultureAttempt(c) => format!(
                "{} using Arts",
                format_cultural_influence_attempt_log_item(
                    game,
                    player.index,
                    player_name,
                    c,
                    &CustomActionType::ArtsInfluenceCultureAttempt.playing_action_type()
                )
            ),
            CustomAction::VotingIncreaseHappiness(i) => format!(
                "{} using Voting",
                format_happiness_increase(player, player_name, i)
            ),
            CustomAction::FreeEconomyCollect(c) => format!(
                "{} using Free Economy",
                format_collect_log_item(player, player_name, c)
            ),
            CustomAction::Taxes(r) => {
                format!("{player_name} paid 1 mood token to collect {r} using Taxes")
            }
        }
    }
}

impl CustomActionType {
    #[must_use]
    pub fn info(&self) -> CustomActionInfo {
        match self {
            CustomActionType::AbsolutePower => {
                self.free_and_once_per_turn(ResourcePile::mood_tokens(2))
            }
            CustomActionType::CivilLiberties | CustomActionType::Sports => self.regular(),
            CustomActionType::Bartering | CustomActionType::Theaters => {
                self.free_and_once_per_turn(ResourcePile::empty())
            }
            CustomActionType::ArtsInfluenceCultureAttempt => {
                self.free_and_once_per_turn(ResourcePile::culture_tokens(1))
            }
            CustomActionType::VotingIncreaseHappiness => self.cost(ResourcePile::mood_tokens(1)),
            CustomActionType::FreeEconomyCollect | CustomActionType::ForcedLabor => {
                self.free_and_once_per_turn(ResourcePile::mood_tokens(1))
            }
            CustomActionType::Taxes => self.once_per_turn(ResourcePile::mood_tokens(1)),
        }
    }

    #[must_use]
    pub fn is_available(&self, game: &Game, player_index: usize) -> bool {
        self.playing_action_type()
            .is_available(game, player_index)
            .is_ok()
    }

    #[must_use]
    pub fn is_available_city(&self, player: &Player, city: &City) -> bool {
        match self {
            CustomActionType::Sports => {
                sports_options(city).is_some_and(|c| c.can_afford(&player.resources))
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn is_city_bound(&self) -> bool {
        matches!(self, CustomActionType::Sports)
    }

    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        PlayingActionType::Custom(self.info())
    }

    #[must_use]
    fn regular(&self) -> CustomActionInfo {
        CustomActionInfo::new(self, false, false, ResourcePile::empty())
    }

    #[must_use]
    fn cost(&self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(self, true, false, cost)
    }

    #[must_use]
    fn once_per_turn(&self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(self, false, true, cost)
    }

    #[must_use]
    fn free_and_once_per_turn(&self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(self, true, true, cost)
    }

    #[must_use]
    pub(crate) fn execute_builtin(&self) -> Builtin {
        match self {
            CustomActionType::Sports => use_sports(),
            CustomActionType::Theaters => use_theaters(),
            CustomActionType::Bartering => use_bartering(),
            _ => {
                panic!("CustomActionType::execute_builtin called on non-builtin action")
            }
        }
    }
}

pub(crate) fn execute_custom_action(
    game: &mut Game,
    player_index: usize,
    action: CustomEventAction,
) {
    let _ = game.trigger_persistent_event_with_listener(
        &[player_index],
        |e| &mut e.custom_action,
        &action.action.execute_builtin().listeners,
        action,
        PersistentEventType::CustomAction,
        None,
        |_| {},
    );
}
