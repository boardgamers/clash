use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};

use crate::action_card::{ActionCardInfo, land_battle_won_action, play_action_card};
use crate::advance::{Advance, gain_advance_without_payment};
use crate::city::{MoodState, found_city};
use crate::collect::{PositionCollection, collect};
use crate::construct::Construct;
use crate::content::custom_actions::{CustomActionType, CustomEventAction, execute_custom_action};
use crate::cultural_influence::{InfluenceCultureAttempt, influence_culture_attempt};
use crate::game::GameState;
use crate::happiness::increase_happiness;
use crate::player::{Player, remove_unit};
use crate::player_events::PlayingActionInfo;
use crate::recruit::recruit;
use crate::unit::Units;
use crate::wonder::{WonderCardInfo, WonderDiscount, cities_for_wonder, on_play_wonder_card};
use crate::{game::Game, position::Position, resource_pile::ResourcePile};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<PositionCollection>,
    pub total: ResourcePile,
    pub action_type: PlayingActionType,
}

impl Collect {
    #[must_use]
    pub fn new(
        city_position: Position,
        collections: Vec<PositionCollection>,
        total: ResourcePile,
        action_type: PlayingActionType,
    ) -> Self {
        Self {
            city_position,
            collections,
            total,
            action_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Recruit {
    pub units: Units,
    pub city_position: Position,
    pub payment: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_name: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaced_units: Vec<u32>,
}

impl Recruit {
    #[must_use]
    pub fn new(units: &Units, city_position: Position, payment: ResourcePile) -> Self {
        Self {
            units: units.clone(),
            city_position,
            payment,
            leader_name: None,
            replaced_units: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_leader(mut self, leader_name: &str) -> Self {
        self.leader_name = Some(leader_name.to_string());
        self
    }

    #[must_use]
    pub fn with_replaced_units(mut self, replaced_units: &[u32]) -> Self {
        self.replaced_units = replaced_units.to_vec();
        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct IncreaseHappiness {
    pub happiness_increases: Vec<(Position, u8)>,
    pub payment: ResourcePile,
    pub action_type: PlayingActionType,
}

impl IncreaseHappiness {
    #[must_use]
    pub fn new(
        happiness_increases: Vec<(Position, u8)>,
        payment: ResourcePile,
        action_type: PlayingActionType,
    ) -> Self {
        Self {
            happiness_increases,
            payment,
            action_type,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum PlayingActionType {
    Advance,
    FoundCity,
    Construct,
    Collect,
    Recruit,
    MoveUnits,
    IncreaseHappiness,
    InfluenceCultureAttempt,
    ActionCard(u8),
    WonderCard(String),
    Custom(CustomActionType),
    EndTurn,
}

impl PlayingActionType {
    ///
    /// # Errors
    /// Returns an error if the action is not available
    pub fn is_available(&self, game: &Game, player_index: usize) -> Result<(), String> {
        if !game.events.is_empty() || game.state != GameState::Playing {
            return Err("Game is not in playing state".to_string());
        }

        self.cost(game).is_available(game, player_index)?;

        let p = game.player(player_index);

        match self {
            PlayingActionType::Custom(c) => {
                let info = c.info();
                if !p.custom_actions.contains_key(c) {
                    return Err("Custom action not available".to_string());
                }

                if info.once_per_turn && p.played_once_per_turn_actions.contains(c) {
                    return Err("Custom action already played this turn".to_string());
                }

                let can_play = match c {
                    CustomActionType::Bartering => !p.action_cards.is_empty(),
                    CustomActionType::Sports => p.resources.culture_tokens > 0 && any_non_happy(p),
                    CustomActionType::Theaters => {
                        p.resources.culture_tokens > 0 || p.resources.mood_tokens > 0
                    }
                    CustomActionType::ForcedLabor => any_angry(p),
                    _ => true,
                };
                if !can_play {
                    return Err("Cannot play custom action".to_string());
                }
            }
            PlayingActionType::ActionCard(id) => {
                if !p.action_cards.contains(id) {
                    return Err("Action card not available".to_string());
                }

                let civil_card = game.cache.get_civil_card(*id);
                let mut satisfying_action: Option<usize> = None;
                if civil_card.requirement_land_battle_won {
                    if let Some(action_log_index) = land_battle_won_action(game, player_index, *id)
                    {
                        satisfying_action = Some(action_log_index);
                    } else {
                        return Err("Requirement not met".to_string());
                    }
                }
                if !(civil_card.can_play)(
                    game,
                    p,
                    &ActionCardInfo::new(*id, satisfying_action, None),
                ) {
                    return Err("Cannot play action card".to_string());
                }
            }
            PlayingActionType::WonderCard(name) => {
                if !p.wonder_cards.contains(name) {
                    return Err("Wonder card not available".to_string());
                }

                if cities_for_wonder(name, game, p, &WonderDiscount::default()).is_empty() {
                    return Err("no cities for wonder".to_string());
                }
            }
            _ => {}
        }

        let mut possible = Ok(());
        p.trigger_event(
            |e| &e.is_playing_action_available,
            &mut possible,
            game,
            &PlayingActionInfo {
                player: player_index,
                action_type: self.clone(),
            },
        );
        possible
    }

    #[must_use]
    pub fn cost(&self, game: &Game) -> ActionCost {
        match self {
            PlayingActionType::Custom(custom_action) => custom_action.info().action_type.clone(),
            PlayingActionType::ActionCard(id) => game.cache.get_civil_card(*id).action_type.clone(),
            PlayingActionType::EndTurn => ActionCost::cost(ResourcePile::empty()),
            _ => ActionCost::regular(),
        }
    }

    #[must_use]
    pub fn remaining_resources(&self, p: &Player, game: &Game) -> ResourcePile {
        let mut r = p.resources.clone();
        r -= self.cost(game).cost.clone();
        r
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum PlayingAction {
    Advance {
        advance: Advance,
        payment: ResourcePile,
    },
    FoundCity {
        settler: u32,
    },
    Construct(Construct),
    Collect(Collect),
    Recruit(Recruit),
    IncreaseHappiness(IncreaseHappiness),
    InfluenceCultureAttempt(InfluenceCultureAttempt),
    Custom(CustomEventAction),
    ActionCard(u8),
    WonderCard(String),
    EndTurn,
}

impl PlayingAction {
    pub(crate) fn execute(
        self,
        game: &mut Game,
        player_index: usize,
        redo: bool,
    ) -> Result<(), String> {
        use crate::construct;
        use PlayingAction::*;
        let playing_action_type = self.playing_action_type();
        if !redo {
            playing_action_type.is_available(game, player_index)?;
        }
        playing_action_type.cost(game).pay(game, player_index);

        if let PlayingActionType::Custom(c) = playing_action_type {
            if c.info().once_per_turn {
                game.players[player_index]
                    .played_once_per_turn_actions
                    .push(c);
            }
        }

        match self {
            Advance { advance, payment } => {
                if !game.player(player_index).can_advance(advance, game) {
                    return Err("Cannot advance".to_string());
                }
                game.player(player_index)
                    .advance_cost(advance, game, game.execute_cost_trigger())
                    .pay(game, &payment);
                gain_advance_without_payment(game, advance, player_index, payment, true);
            }
            FoundCity { settler } => {
                let settler = remove_unit(player_index, settler, game);
                if !settler.can_found_city(game) {
                    return Err("Cannot found city".to_string());
                }
                found_city(game, player_index, settler.position);
            }
            Construct(c) => construct::construct(game, player_index, &c)?,
            Collect(c) => collect(game, player_index, &c)?,
            Recruit(r) => recruit(game, player_index, r)?,
            IncreaseHappiness(i) => {
                increase_happiness(game, player_index, &i.happiness_increases, Some(i.payment));
            }
            InfluenceCultureAttempt(c) => {
                influence_culture_attempt(game, player_index, &c.selected_structure)?;
            }
            ActionCard(a) => play_action_card(game, player_index, a),
            WonderCard(name) => {
                on_play_wonder_card(
                    game,
                    player_index,
                    WonderCardInfo::new(name, WonderDiscount::default()),
                );
            }
            Custom(custom_action) => {
                execute_custom_action(game, player_index, custom_action);
            }
            EndTurn => game.next_turn(),
        }
        Ok(())
    }

    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        match self {
            PlayingAction::Advance { .. } => PlayingActionType::Advance,
            PlayingAction::FoundCity { .. } => PlayingActionType::FoundCity,
            PlayingAction::Construct(_) => PlayingActionType::Construct,
            PlayingAction::Collect(c) => allowed_types(
                &c.action_type,
                &[
                    PlayingActionType::Collect,
                    PlayingActionType::Custom(CustomActionType::FreeEconomyCollect),
                ],
            ),
            PlayingAction::Recruit(_) => PlayingActionType::Recruit,
            PlayingAction::IncreaseHappiness(h) => allowed_types(
                &h.action_type,
                &[
                    PlayingActionType::IncreaseHappiness,
                    PlayingActionType::Custom(CustomActionType::VotingIncreaseHappiness),
                ],
            ),
            PlayingAction::InfluenceCultureAttempt(i) => allowed_types(
                &i.action_type,
                &[
                    PlayingActionType::InfluenceCultureAttempt,
                    PlayingActionType::Custom(CustomActionType::ArtsInfluenceCultureAttempt),
                ],
            ),
            PlayingAction::ActionCard(a) => PlayingActionType::ActionCard(*a),
            PlayingAction::WonderCard(name) => PlayingActionType::WonderCard(name.clone()),
            PlayingAction::Custom(c) => PlayingActionType::Custom(c.action.clone()),
            PlayingAction::EndTurn => PlayingActionType::EndTurn,
        }
    }
}

fn allowed_types(
    playing_action_type: &PlayingActionType,
    allowed: &[PlayingActionType],
) -> PlayingActionType {
    assert!(allowed.iter().any(|a| a == playing_action_type));
    playing_action_type.clone()
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct ActionCost {
    pub free: bool,
    pub cost: ResourcePile,
}

impl ActionCost {
    pub(crate) fn is_available(&self, game: &Game, player_index: usize) -> Result<(), String> {
        let p = game.player(player_index);
        if !p.resources.has_at_least(&self.cost) {
            return Err("Not enough resources for action type".to_string());
        }

        if !(self.free || game.actions_left > 0) {
            return Err("No actions left".to_string());
        }
        Ok(())
    }

    pub(crate) fn pay(&self, game: &mut Game, player_index: usize) {
        let p = game.player_mut(player_index);
        let cost = self.cost.clone();
        p.lose_resources(cost.clone());
        if !self.free {
            game.actions_left -= 1;
        }
    }
}

impl ActionCost {
    #[must_use]
    pub fn cost(cost: ResourcePile) -> Self {
        Self::new(true, cost)
    }

    #[must_use]
    pub fn regular() -> Self {
        Self::new(false, ResourcePile::empty())
    }

    #[must_use]
    pub fn regular_with_cost(cost: ResourcePile) -> Self {
        Self::new(false, cost)
    }

    #[must_use]
    pub fn free() -> Self {
        Self::new(true, ResourcePile::empty())
    }

    #[must_use]
    pub fn new(free: bool, cost: ResourcePile) -> Self {
        Self { free, cost }
    }
}

pub(crate) fn roll_boost_cost(roll: u8) -> ResourcePile {
    ResourcePile::culture_tokens(5 - roll)
}

#[must_use]
pub fn base_and_custom_action(
    actions: Vec<PlayingActionType>,
) -> (Option<PlayingActionType>, Option<CustomActionType>) {
    let (mut custom, mut action): (Vec<_>, Vec<_>) = actions.into_iter().partition_map(|a| {
        if let PlayingActionType::Custom(c) = a {
            Either::Left(c.clone())
        } else {
            Either::Right(a.clone())
        }
    });
    (action.pop(), custom.pop())
}

#[must_use]
pub(crate) fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: &CustomActionType,
) -> Vec<PlayingActionType> {
    vec![action, custom.playing_action_type()]
        .into_iter()
        .filter_map(|a| a.is_available(game, player).map(|()| a).ok())
        .collect()
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

// None if the action cost has been paid
#[must_use]
pub fn remaining_resources_for_action(
    game: &Game,
    action_type: Option<&PlayingActionType>,
    player: &Player,
) -> ResourcePile {
    action_type.map_or_else(
        || player.resources.clone(),
        |action_type| action_type.remaining_resources(player, game),
    )
}
