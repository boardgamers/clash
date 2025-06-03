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
use std::fmt::Debug;
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
pub struct CustomActionCost {
    pub action_type: ActionCost,
    pub once_per_turn: Option<CustomActionType>,
}

impl CustomActionCost {
    #[must_use]
    pub(crate) fn new(
        free: bool,
        once_per_turn: Option<CustomActionType>,
        cost: ActionResourceCost,
    ) -> CustomActionCost {
        CustomActionCost {
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
    pub info: CustomActionCost,
}

impl CustomActionCommand {
    #[must_use]
    pub fn new(
        action: CustomActionType,
        execution: CustomActionExecution,
        event_origin: EventOrigin,
        info: CustomActionCost,
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
    pub ability: Ability,
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
            ability: execution,
            city_checker,
        }
    }
}

#[derive(Clone)]
pub enum CustomActionExecution {
    Modifier((PlayingActionType, String)),
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
    pub(crate) fn regular() -> CustomActionCost {
        CustomActionCost::new(false, None, ActionResourceCost::free())
    }

    #[must_use]
    pub(crate) fn cost(cost: ResourcePile) -> CustomActionCost {
        CustomActionCost::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn once_per_turn(self, cost: ResourcePile) -> CustomActionCost {
        CustomActionCost::new(false, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free(cost: ResourcePile) -> CustomActionCost {
        CustomActionCost::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free_and_once_per_turn(self, cost: ResourcePile) -> CustomActionCost {
        CustomActionCost::new(true, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub(crate) fn free_and_once_per_turn_mutually_exclusive(
        cost: ResourcePile,
        mutually_exclusive: CustomActionType,
    ) -> CustomActionCost {
        CustomActionCost::new(
            true,
            Some(mutually_exclusive),
            ActionResourceCost::resources(cost),
        )
    }

    #[must_use]
    pub(crate) fn free_and_advance_cost_without_discounts() -> CustomActionCost {
        CustomActionCost::new(true, None, ActionResourceCost::AdvanceCostWithoutDiscount)
    }
}

pub(crate) fn execute_custom_action(
    game: &mut Game,
    player_index: usize,
    a: CustomActionActivation,
) {
    let _ = trigger_persistent_event_with_listener(
        game,
        &[player_index],
        |e| &mut e.custom_action,
        &custom_action_execution(game.player(player_index), a.action.action)
            .ability
            .listeners,
        a,
        PersistentEventType::CustomAction,
        TriggerPersistentEventParams::default(),
    );
}

pub(crate) fn custom_action_execution(
    player: &Player,
    action_type: CustomActionType,
) -> CustomActionActionExecution {
    let CustomActionExecution::Action(e) = player.custom_action_command(action_type).execution
    else {
        panic!("Custom action {action_type:?} is not an action");
    };
    e
}

pub(crate) fn custom_action_modifier_name(
    player: &Player,
    action_type: CustomActionType,
) -> String {
    let CustomActionExecution::Modifier((_, name)) =
        player.custom_action_command(action_type).execution
    else {
        panic!("Custom action {action_type:?} is not a modifier");
    };
    name
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
            if let CustomActionExecution::Modifier((t, _)) = &p.custom_action_command(*c).execution
            {
                t == action_type
            } else {
                false
            }
        }
        action_type => action_type == base_type,
    }
}
