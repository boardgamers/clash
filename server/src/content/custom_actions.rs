use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::city::City;
use crate::content::advances::autocracy::{use_absolute_power, use_forced_labor};
use crate::content::advances::culture::{sports_options, use_sports, use_theaters};
use crate::content::advances::democracy::use_civil_liberties;
use crate::content::advances::economy::{use_bartering, use_taxes};
use crate::content::builtin::Builtin;
use crate::content::persistent_events::PersistentEventType;
use crate::player::Player;
use crate::playing_actions::PlayingActionType;
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

#[derive(Clone, Debug, PartialEq)]
pub struct CustomActionInfo {
    pub action_type: ActionCost,
    pub once_per_turn: bool,
}

impl CustomActionInfo {
    #[must_use]
    fn new(free: bool, once_per_turn: bool, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo {
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

impl CustomActionType {
    #[must_use]
    pub fn info(&self) -> CustomActionInfo {
        match self {
            CustomActionType::AbsolutePower => {
                CustomActionType::free_and_once_per_turn(ResourcePile::mood_tokens(2))
            }
            CustomActionType::CivilLiberties | CustomActionType::Sports => CustomActionType::regular(),
            CustomActionType::Bartering | CustomActionType::Theaters => {
                CustomActionType::free_and_once_per_turn(ResourcePile::empty())
            }
            CustomActionType::ArtsInfluenceCultureAttempt => {
                CustomActionType::free_and_once_per_turn(ResourcePile::culture_tokens(1))
            }
            CustomActionType::VotingIncreaseHappiness => CustomActionType::cost(ResourcePile::mood_tokens(1)),
            CustomActionType::FreeEconomyCollect | CustomActionType::ForcedLabor => {
                CustomActionType::free_and_once_per_turn(ResourcePile::mood_tokens(1))
            }
            CustomActionType::Taxes => CustomActionType::once_per_turn(ResourcePile::mood_tokens(1)),
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
        PlayingActionType::Custom(self.clone())
    }

    #[must_use]
    fn regular() -> CustomActionInfo {
        CustomActionInfo::new(false, false, ResourcePile::empty())
    }

    #[must_use]
    fn cost(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, false, cost)
    }

    #[must_use]
    fn once_per_turn(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(false, true, cost)
    }

    #[must_use]
    fn free_and_once_per_turn(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, true, cost)
    }
}

pub(crate) fn custom_action_builtins() -> HashMap<CustomActionType, Builtin> {
    HashMap::from([
        (CustomActionType::AbsolutePower, use_absolute_power()),
        (CustomActionType::ForcedLabor, use_forced_labor()),
        (CustomActionType::CivilLiberties, use_civil_liberties()),
        (CustomActionType::Bartering, use_bartering()),
        (CustomActionType::Sports, use_sports()),
        (CustomActionType::Taxes, use_taxes()),
        (CustomActionType::Theaters, use_theaters()),
    ])
}

pub(crate) fn execute_custom_action(
    game: &mut Game,
    player_index: usize,
    action: CustomEventAction,
) {
    let _ = game.trigger_persistent_event_with_listener(
        &[player_index],
        |e| &mut e.custom_action,
        &custom_action_builtins()[&action.action.clone()].listeners,
        action,
        PersistentEventType::CustomAction,
        None,
        |_| {},
    );
}
