use crate::action_cost::ActionCostOncePerTurn;
use crate::city::{City, MoodState};
use crate::content::ability::Ability;
use crate::content::persistent_events::{
    PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_with_listener,
};
use crate::events::{EventOrigin, EventPlayer};
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
pub struct CustomActionInfo {
    pub action: CustomActionType,
    pub execution: CustomActionExecution,
    pub event_origin: EventOrigin,
    pub cost: ActionCostOncePerTurn,
}

impl CustomActionInfo {
    #[must_use]
    pub fn new(
        action: CustomActionType,
        execution: CustomActionExecution,
        event_origin: EventOrigin,
        cost: ActionCostOncePerTurn,
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

pub(crate) fn log_start_custom_action(game: &mut Game, player_index: usize, action: &CustomAction) {
    let p = game.player(player_index);
    let player = EventPlayer::from_player(player_index, game, action.action.playing_action_type().origin(p));
    if let Some(city) = action.city {
        player.log(
            game,
            &format!("Start action in city {city}"),
        )
    } else {
        player.log(game, "Start action")
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
    if let CustomActionExecution::Action(e) =
        player.custom_actions.get(&action_type)?.execution.clone()
    {
        Some(e)
    } else {
        None
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
            if let CustomActionExecution::Modifier(t) = &p.custom_action_info(*c).execution {
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
    if let PlayingActionType::Custom(c) = action_type {
        c.playing_action_type().origin(player)
    } else {
        base_action_origin
    }
}
