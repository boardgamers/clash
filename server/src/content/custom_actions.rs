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

pub(crate) struct CustomActionOncePerTurn {
    action: CustomActionType,
}

impl CustomActionOncePerTurn {
    #[must_use]
    pub fn new(action: CustomActionType) -> Self {
        Self { action }
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn any_times(self) -> CustomActionActionCost {
        CustomActionActionCost::new(None)
    }

    #[must_use]
    pub fn once_per_turn(self) -> CustomActionActionCost {
        CustomActionActionCost::new(Some(self.action))
    }

    #[allow(clippy::unused_self)]
    #[must_use]
    pub fn once_per_turn_mutually_exclusive(
        self,
        mutually_exclusive: CustomActionType,
    ) -> CustomActionActionCost {
        CustomActionActionCost::new(Some(mutually_exclusive))
    }
}

pub(crate) struct CustomActionActionCost {
    once_per_turn: Option<CustomActionType>,
}

impl CustomActionActionCost {
    #[must_use]
    fn new(once_per_turn: Option<CustomActionType>) -> Self {
        Self { once_per_turn }
    }

    #[must_use]
    pub fn action(self) -> CustomActionResourceCost {
        CustomActionResourceCost::new(self.once_per_turn, false)
    }

    #[must_use]
    pub fn free_action(self) -> CustomActionResourceCost {
        CustomActionResourceCost::new(self.once_per_turn, true)
    }
}

pub(crate) struct CustomActionResourceCost {
    once_per_turn: Option<CustomActionType>,
    free: bool,
}

impl CustomActionResourceCost {
    #[must_use]
    fn new(once_per_turn: Option<CustomActionType>, free: bool) -> CustomActionResourceCost {
        CustomActionResourceCost {
            once_per_turn,
            free,
        }
    }

    #[must_use]
    pub fn no_resources(self) -> CustomActionCost {
        CustomActionCost::new(self.free, self.once_per_turn, ActionResourceCost::free())
    }

    #[must_use]
    pub fn resources(self, cost: ResourcePile) -> CustomActionCost {
        CustomActionCost::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::resources(cost),
        )
    }

    #[must_use]
    pub fn tokens(self, cost: u8) -> CustomActionCost {
        CustomActionCost::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::tokens(cost),
        )
    }

    #[must_use]
    pub fn advance_cost_without_discounts(self) -> CustomActionCost {
        CustomActionCost::new(
            self.free,
            self.once_per_turn,
            ActionResourceCost::AdvanceCostWithoutDiscount,
        )
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
pub struct CustomActionInfo {
    pub action: CustomActionType,
    pub execution: CustomActionExecution,
    pub event_origin: EventOrigin,
    pub cost: CustomActionCost,
}

impl CustomActionInfo {
    #[must_use]
    pub fn new(
        action: CustomActionType,
        execution: CustomActionExecution,
        event_origin: EventOrigin,
        cost: CustomActionCost,
    ) -> CustomActionInfo {
        CustomActionInfo {
            action,
            execution,
            event_origin,
            cost,
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
    ArtOfWar,
    AgricultureEconomist,

    // Vikings
    Danegeld,
    LegendaryExplorer,
    NewColonies,
}

impl CustomActionType {
    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        PlayingActionType::Custom(*self)
    }
}

pub(crate) fn execute_custom_action(
    game: &mut Game,
    player_index: usize,
    a: CustomActionActivation,
) {
    let action = a.action.clone();
    let p = game.player(player_index);
    let action_type = action.action;
    let name = custom_action_execution(p, action_type)
        .unwrap_or_else(|| panic!("Custom action {action_type:?} is not an action"))
        .ability
        .name;
    game.add_info_log_item(&format!(
        "{p} started {name}{}",
        if let Some(p) = action.city {
            format!(" at {p}")
        } else {
            String::new()
        }
    ));
    on_custom_action(game, player_index, a);
}

pub(crate) fn on_custom_action(game: &mut Game, player_index: usize, a: CustomActionActivation) {
    let Some(execution) = custom_action_execution(game.player(player_index), a.action.action)
    else {
        // may have disappeared due to a game change,
        // such as Erik the Red unloading units and being killed
        game.events.pop();
        return;
    };
    let _ = trigger_persistent_event_with_listener(
        game,
        &[player_index],
        |e| &mut e.custom_action,
        &execution.ability.listeners,
        a,
        PersistentEventType::CustomAction,
        TriggerPersistentEventParams::default(),
    );
}

pub(crate) fn custom_action_execution(
    player: &Player,
    action_type: CustomActionType,
) -> Option<CustomActionActionExecution> {
    if let CustomActionExecution::Action(e) =
        player.custom_actions.get(&action_type)?.execution.clone()
    {
        Some(e)
    } else {
        None
    }
}

pub(crate) fn custom_action_modifier_name(
    player: &Player,
    action_type: CustomActionType,
) -> String {
    match player.custom_action_info(action_type).execution {
        CustomActionExecution::Modifier((_, name)) => name.clone(),
        // Sports is not a modifier, but is shown for logging purposes as a modifier
        CustomActionExecution::Action(a) => a.ability.name.clone(),
    }
}

pub(crate) fn can_play_custom_action(
    game: &Game,
    p: &Player,
    c: CustomActionType,
) -> Result<(), String> {
    if !p.custom_actions.contains_key(&c) {
        return Err("Custom action not available".to_string());
    }

    let info = p.custom_action_info(c);
    if let Some(key) = info.cost.once_per_turn {
        if p.played_once_per_turn_actions.contains(&key) {
            return Err("Custom action already played this turn".to_string());
        }
    }

    if let CustomActionExecution::Action(e) = &info.execution {
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
            if let CustomActionExecution::Modifier((t, _)) = &p.custom_action_info(*c).execution {
                t == action_type
            } else {
                false
            }
        }
        action_type => action_type == base_type,
    }
}
