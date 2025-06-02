use crate::city::{City, MoodState};
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_with_listener,
};
use crate::events::EventOrigin;
use crate::player::Player;
use crate::playing_actions::{ActionResourceCost, PlayingActionType};
use crate::position::Position;
use crate::{game::Game, playing_actions::ActionCost, resource_pile::ResourcePile};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::sync::Arc;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct CustomAction {
    pub action: CustomActionType,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<Position>,
}

impl CustomAction {
    #[must_use]
    pub fn new(action: CustomActionType, city: Option<Position>) -> Self {
        Self { action, city }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct CustomActionActivation {
    #[serde(flatten)]
    pub action: CustomAction,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub payment: ResourcePile,
}

impl CustomActionActivation {
    #[must_use]
    pub fn new(action: CustomAction, payment: ResourcePile) -> Self {
        Self { action, payment }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomActionInfo {
    pub action_type: ActionCost,
    pub once_per_turn: Option<CustomActionType>,
}

impl CustomActionInfo {
    #[must_use]
    pub(crate) fn new(
        free: bool,
        once_per_turn: Option<CustomActionType>,
        cost: ActionResourceCost,
    ) -> CustomActionInfo {
        CustomActionInfo {
            action_type: ActionCost::new(free, cost),
            once_per_turn,
        }
    }
}

#[derive(Clone)]
pub struct CustomActionCommand {
    pub action: CustomActionType,
    pub execution: CustomActionExecution,
    pub event_origin: EventOrigin,
    pub info: CustomActionInfo,
}

impl CustomActionCommand {
    #[must_use]
    pub fn new(
        action: CustomActionType,
        execution: CustomActionExecution,
        event_origin: EventOrigin,
        info: CustomActionInfo,
    ) -> CustomActionCommand {
        CustomActionCommand {
            action,
            execution,
            event_origin,
            info,
        }
    }

    #[must_use]
    pub fn is_city_available(&self, game: &Game, city: &City) -> bool {
        self.city_bound().is_some_and(|checker| checker(game, city))
    }

    #[must_use]
    pub fn city_bound(&self) -> Option<&CustomActionCityChecker> {
        if let CustomActionExecution::Action(a) = &self.execution {
            a.city_checker.as_ref()
        } else {
            None
        }
    }
}

type CustomActionChecker = Arc<dyn Fn(&Game, &Player) -> bool + Sync + Send>;
type CustomActionCityChecker = Arc<dyn Fn(&Game, &City) -> bool + Sync + Send>;

#[derive(Clone)]
pub struct CustomActionActionExecution {
    pub checker: CustomActionChecker,
    pub execution: Ability,
    pub city_checker: Option<CustomActionCityChecker>,
}

impl CustomActionActionExecution {
    #[must_use]
    pub fn new(
        execution: Ability,
        checker: CustomActionChecker,
        city_checker: Option<CustomActionCityChecker>,
    ) -> Self {
        Self {
            checker,
            execution,
            city_checker,
        }
    }
}

#[derive(Clone)]
pub enum CustomActionExecution {
    Modifier(PlayingActionType),
    Action(CustomActionActionExecution),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum CustomActionType {
    // Advances
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

    // Wonders
    GreatLibrary,
    GreatLighthouse,
    GreatStatue,

    // Civilizations,
    // Rome
    Aqueduct,
    Princeps,
    StatesmanIncreaseHappiness,

    // Greece
    HellenisticInfluenceCultureAttempt,
    Idol,
    Master,

    // China
    ImperialArmy,
}

impl CustomActionType {
    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        PlayingActionType::Custom(*self)
    }

    #[must_use]
    pub(crate) fn regular() -> CustomActionInfo {
        CustomActionInfo::new(false, None, ActionResourceCost::free())
    }

    #[must_use]
    pub(crate) fn cost(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn once_per_turn(self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(false, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free_and_once_per_turn(self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free_and_once_per_turn_mutually_exclusive(
        cost: ResourcePile,
        mutually_exclusive: CustomActionType,
    ) -> CustomActionInfo {
        CustomActionInfo::new(
            true,
            Some(mutually_exclusive),
            ActionResourceCost::resources(cost),
        )
    }

    #[must_use]
    pub(crate) fn free_and_advance_cost_without_discounts() -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::AdvanceCostWithoutDiscount)
    }
}

impl Display for CustomActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomActionType::AbsolutePower => write!(f, "Absolute Power"),
            CustomActionType::ForcedLabor => write!(f, "Forced Labor"),
            CustomActionType::CivilLiberties => write!(f, "Civil Liberties"),
            CustomActionType::Bartering => write!(f, "Bartering"),
            CustomActionType::ArtsInfluenceCultureAttempt => {
                write!(f, "Arts")
            }
            CustomActionType::VotingIncreaseHappiness => {
                write!(f, "Voting")
            }
            CustomActionType::FreeEconomyCollect => write!(f, "Free Economy"),
            CustomActionType::Sports => write!(f, "Sports"),
            CustomActionType::Taxes => write!(f, "Taxes"),
            CustomActionType::Theaters => write!(f, "Theaters"),
            CustomActionType::GreatLibrary => write!(f, "Great Library"),
            CustomActionType::GreatLighthouse => write!(f, "Great Lighthouse"),
            CustomActionType::GreatStatue => write!(f, "Great Statue"),
            CustomActionType::Aqueduct => write!(f, "Aqueduct"),
            CustomActionType::Princeps => write!(f, "Princeps"),
            CustomActionType::StatesmanIncreaseHappiness => write!(f, "Statesman"),
            CustomActionType::HellenisticInfluenceCultureAttempt => {
                write!(f, "Hellenistic Culture")
            }
            CustomActionType::Idol => write!(f, "Idol"),
            CustomActionType::Master => write!(f, "Master"),
            CustomActionType::ImperialArmy => write!(f, "Imperial Army"),
        }
    }
}

pub(crate) fn execute_custom_action(
    game: &mut Game,
    player_index: usize,
    a: CustomActionActivation,
) {
    let CustomActionExecution::Action(e) = game
        .player(player_index)
        .custom_action_command(a.action.action)
        .execution
    else {
        panic!("Custom action {:?} is not an action", &a.action);
    };
    let _ = trigger_persistent_event_with_listener(
        game,
        &[player_index],
        |e| &mut e.custom_action,
        &e.execution.listeners,
        a,
        PersistentEventType::CustomAction,
        TriggerPersistentEventParams::default(),
    );
}

pub(crate) fn can_play_custom_action(
    game: &Game,
    p: &Player,
    c: CustomActionType,
) -> Result<(), String> {
    if !p.custom_actions.contains_key(&c) {
        return Err("Custom action not available".to_string());
    }

    let command = p.custom_action_command(c);
    if let Some(key) = command.info.once_per_turn {
        if p.played_once_per_turn_actions.contains(&key) {
            return Err("Custom action already played this turn".to_string());
        }
    }

    if let CustomActionExecution::Action(e) = &command.execution {
        if !(e.checker)(game, p) {
            return Err("Custom action cannot be played".to_string());
        }
    }
    Ok(())
}

pub(crate) fn any_non_happy(player: &Player) -> bool {
    player
        .cities
        .iter()
        .any(|city| city.mood_state != MoodState::Happy)
}

pub(crate) fn is_base_or_modifier(
    action_type: &PlayingActionType,
    p: &Player,
    base_type: &PlayingActionType,
) -> bool {
    match base_type {
        PlayingActionType::Custom(c) => {
            if let CustomActionExecution::Modifier(t) = &p.custom_action_command(*c).execution {
                t == action_type
            } else {
                false
            }
        }
        action_type => action_type == base_type,
    }
}
