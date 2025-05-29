use crate::advance::{Advance, base_advance_cost};
use crate::city::{City, MoodState};
use crate::content::advances::autocracy::{use_absolute_power, use_forced_labor};
use crate::content::advances::culture::{sports_options, use_sports, use_theaters};
use crate::content::advances::democracy::use_civil_liberties;
use crate::content::advances::economy::{use_bartering, use_taxes};
use crate::content::builtin::Builtin;
use crate::content::civilizations::rome::{use_aqueduct, use_princeps};
use crate::content::persistent_events::{
    PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_with_listener,
};
use crate::content::wonders::{
    great_lighthouse_city, great_lighthouse_spawns, use_great_library, use_great_lighthouse,
    use_great_statue,
};
use crate::leader::leader_position;
use crate::player::Player;
use crate::playing_actions::{ActionResourceCost, PlayingActionType};
use crate::position::Position;
use crate::wonder::Wonder;
use crate::{game::Game, playing_actions::ActionCost, resource_pile::ResourcePile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

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
    fn new(
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
}

impl CustomActionType {
    #[must_use]
    pub fn info(&self) -> CustomActionInfo {
        match self {
            CustomActionType::AbsolutePower => {
                self.free_and_once_per_turn(ResourcePile::mood_tokens(2))
            }
            CustomActionType::CivilLiberties | CustomActionType::Sports => {
                CustomActionType::regular()
            }
            CustomActionType::GreatLighthouse | CustomActionType::StatesmanIncreaseHappiness => {
                CustomActionType::free(ResourcePile::empty())
            }
            CustomActionType::Bartering
            | CustomActionType::Theaters
            | CustomActionType::GreatLibrary
            | CustomActionType::GreatStatue => self.free_and_once_per_turn(ResourcePile::empty()),
            CustomActionType::ArtsInfluenceCultureAttempt => {
                self.free_and_once_per_turn(ResourcePile::culture_tokens(1))
            }
            CustomActionType::HellenisticInfluenceCultureAttempt => {
                CustomActionType::free_and_once_per_turn_mutually_exclusive(
                    ResourcePile::mood_tokens(2),
                    CustomActionType::ArtsInfluenceCultureAttempt,
                )
            }
            CustomActionType::VotingIncreaseHappiness => {
                CustomActionType::cost(ResourcePile::mood_tokens(1))
            }
            CustomActionType::Princeps => CustomActionType::cost(ResourcePile::culture_tokens(1)),
            CustomActionType::FreeEconomyCollect | CustomActionType::ForcedLabor => {
                self.free_and_once_per_turn(ResourcePile::mood_tokens(1))
            }
            CustomActionType::Taxes => self.once_per_turn(ResourcePile::mood_tokens(1)),
            CustomActionType::Aqueduct => {
                CustomActionType::free_and_advance_cost_without_discounts()
            }
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
                sports_options(player, city).is_some_and(|c| c.can_afford(&player.resources))
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
        PlayingActionType::Custom(*self)
    }

    #[must_use]
    fn regular() -> CustomActionInfo {
        CustomActionInfo::new(false, None, ActionResourceCost::free())
    }

    #[must_use]
    fn cost(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    fn once_per_turn(self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(false, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    fn free(cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::resources(cost))
    }

    #[must_use]
    fn free_and_once_per_turn(self, cost: ResourcePile) -> CustomActionInfo {
        CustomActionInfo::new(true, Some(self), ActionResourceCost::resources(cost))
    }

    #[must_use]
    fn free_and_once_per_turn_mutually_exclusive(
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
    fn free_and_advance_cost_without_discounts() -> CustomActionInfo {
        CustomActionInfo::new(true, None, ActionResourceCost::AdvanceCostWithoutDiscount)
    }

    #[must_use]
    pub fn is_modifier(&self) -> bool {
        collect_modifiers().contains(self)
            || influence_modifiers().contains(self)
            || happiness_modifiers().contains(self)
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
        }
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
        (CustomActionType::GreatLibrary, use_great_library()),
        (CustomActionType::GreatLighthouse, use_great_lighthouse()),
        (CustomActionType::GreatStatue, use_great_statue()),
        (CustomActionType::Aqueduct, use_aqueduct()),
        (CustomActionType::Princeps, use_princeps()),
    ])
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
        &custom_action_builtins()[&a.action.action.clone()].listeners,
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

    if let Some(key) = c.info().once_per_turn {
        if p.played_once_per_turn_actions.contains(&key) {
            return Err("Custom action already played this turn".to_string());
        }
    }

    let can_play = match c {
        CustomActionType::Bartering => !p.action_cards.is_empty(),
        CustomActionType::Sports => can_use_sports(p),
        CustomActionType::Theaters => p.resources.culture_tokens > 0 || p.resources.mood_tokens > 0,
        CustomActionType::ForcedLabor => any_angry(p),
        CustomActionType::GreatStatue => !p.objective_cards.is_empty(),
        CustomActionType::GreatLighthouse => {
            great_lighthouse_city(p).can_activate()
                && p.available_units().ships > 0
                && !great_lighthouse_spawns(game, p.index).is_empty()
        }
        CustomActionType::Aqueduct => {
            !p.has_advance(Advance::Sanitation) && p.can_afford(&base_advance_cost(p))
        }
        CustomActionType::Princeps => game
            .try_get_any_city(leader_position(p))
            .is_some_and(City::can_activate),
        _ => true,
    };
    if !can_play {
        return Err("Cannot play custom action".to_string());
    }
    Ok(())
}

fn can_use_sports(p: &Player) -> bool {
    if !any_non_happy(p) {
        return false;
    }
    if p.resources.culture_tokens > 0 {
        return true;
    }
    p.wonders_owned.contains(Wonder::Colosseum) && p.resources.mood_tokens > 0
}

fn any_non_happy(player: &Player) -> bool {
    player
        .cities
        .iter()
        .any(|city| city.mood_state != MoodState::Happy)
}

fn any_angry(player: &Player) -> bool {
    player
        .cities
        .iter()
        .any(|city| city.mood_state == MoodState::Angry)
}

pub(crate) fn influence_modifiers() -> Vec<CustomActionType> {
    vec![
        CustomActionType::ArtsInfluenceCultureAttempt,
        CustomActionType::HellenisticInfluenceCultureAttempt,
    ]
}

pub(crate) fn happiness_modifiers() -> Vec<CustomActionType> {
    vec![
        CustomActionType::VotingIncreaseHappiness,
        CustomActionType::StatesmanIncreaseHappiness,
    ]
}

pub(crate) fn collect_modifiers() -> Vec<CustomActionType> {
    vec![CustomActionType::FreeEconomyCollect]
}
