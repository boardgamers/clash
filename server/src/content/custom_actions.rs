use crate::action_cost::ActionCostOncePerTurn;
use crate::city::{City, MoodState};
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_with_listener,
};
use crate::events::EventOrigin;
use crate::player::Player;
use crate::playing_actions::PlayingActionType;
use crate::position::Position;
use crate::{game::Game, resource_pile::ResourcePile};
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

#[derive(Clone)]
pub struct SpecialActionInfo {
    pub action: SpecialAction,
    pub execution: SpecialActionExecution,
    pub event_origin: EventOrigin,
    pub cost: ActionCostOncePerTurn,
}

impl SpecialActionInfo {
    #[must_use]
    pub fn new(
        action: SpecialAction,
        execution: SpecialActionExecution,
        event_origin: EventOrigin,
        cost: ActionCostOncePerTurn,
    ) -> SpecialActionInfo {
        SpecialActionInfo {
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
        if let SpecialActionExecution::Action(a) = &self.execution {
            a.city_checker.as_ref()
        } else {
            None
        }
    }

    ///
    /// # Panics
    /// Panics if not a modifier action
    #[must_use]
    pub fn custom_action_type(&self) -> CustomActionType {
        if let SpecialAction::Custom(c) = self.action {
            c
        } else {
            panic!("Not a custom action")
        }
    }

    ///
    /// # Panics
    /// Panics if not a modifier action
    #[must_use]
    pub fn modifier_action_type(&self) -> PlayingActionType {
        if let SpecialAction::Modifier(m) = self.action {
            m.playing_action_type()
        } else {
            panic!("Not a modifier action")
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
        ability: Ability,
        checker: CustomActionChecker,
        city_checker: Option<CustomActionCityChecker>,
    ) -> Self {
        Self {
            checker,
            ability,
            city_checker,
        }
    }
}

#[derive(Clone)]
pub enum SpecialActionExecution {
    Modifier(PlayingActionType),
    Action(CustomActionActionExecution),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum SpecialAction {
    Modifier(PlayingActionModifier),
    Custom(CustomActionType),
}

impl SpecialAction {
    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        match self {
            SpecialAction::Modifier(m) => m.playing_action_type(),
            SpecialAction::Custom(c) => c.playing_action_type(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayingActionModifier {
    // Advances
    ArtsInfluenceCultureAttempt,
    FreeEconomyCollect,
    VotingIncreaseHappiness,

    // Update patch Advances
    CityPlanning,
    Philosophy,

    // Civilizations,
    // Rome
    StatesmanIncreaseHappiness,

    // Greece
    HellenisticInfluenceCultureAttempt,
}

impl PlayingActionModifier {
    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        PlayingActionType::Special(SpecialAction::Modifier(*self))
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum CustomActionType {
    // Advances
    AbsolutePower,
    ForcedLabor,
    CivilLiberties,
    Bartering,
    Sports,
    Taxes,
    Theaters,

    // Update patch Advances
    WelfareState,
    ForcedMarch,
    MilitaryState,
    Totalitarianism,

    // Wonders
    GreatLibrary,
    GreatLighthouse,
    GreatStatue,

    // Civilizations,
    // Rome
    Aqueduct,
    Princeps,

    // Greece
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
        PlayingActionType::Special(SpecialAction::Custom(*self))
    }
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
    if let SpecialActionExecution::Action(e) = player
        .special_actions
        .get(&SpecialAction::Custom(action_type))?
        .execution
        .clone()
    {
        Some(e)
    } else {
        None
    }
}

pub(crate) fn can_play_special_action(
    game: &Game,
    p: &Player,
    c: SpecialAction,
) -> Result<(), String> {
    if !p.special_actions.contains_key(&c) {
        return Err("Custom action not available".to_string());
    }

    let info = p.special_action_info(&c);
    if let Some(key) = info.cost.once_per_turn
        && p.played_once_per_turn_actions.contains(&key)
    {
        return Err("Custom action already played this turn".to_string());
    }

    if let SpecialActionExecution::Action(e) = &info.execution
        && !(e.checker)(game, p)
    {
        return Err("Custom action cannot be played".to_string());
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
        PlayingActionType::Special(SpecialAction::Modifier(c)) => {
            if let SpecialActionExecution::Modifier(t) = &p
                .special_action_info(&SpecialAction::Modifier(*c))
                .execution
            {
                t == action_type
            } else {
                false
            }
        }
        action_type => action_type == base_type,
    }
}

pub(crate) fn custom_action_modifier_event_origin(
    base_action_origin: EventOrigin,
    action_type: &PlayingActionType,
    player: &Player,
) -> EventOrigin {
    if let PlayingActionType::Special(SpecialAction::Modifier(c)) = action_type {
        c.playing_action_type().origin(player)
    } else {
        base_action_origin
    }
}
