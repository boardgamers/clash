use serde::{Deserialize, Serialize};

use crate::action_card::play_action_card;
use crate::advance::gain_advance;
use crate::city::MoodState;
use crate::collect::collect;
use crate::construct::Construct;
use crate::content::action_cards::get_civil_card;
use crate::content::advances::get_advance;
use crate::content::custom_actions::CustomActionInfo;
use crate::cultural_influence::influence_culture_attempt;
use crate::game::GameState;
use crate::player::Player;
use crate::player_events::PlayingActionInfo;
use crate::recruit::{recruit, recruit_cost};
use crate::unit::Units;
use crate::wonder::{cities_for_wonder, on_play_wonder_card, WonderCardInfo, WonderDiscount};
use crate::{
    city::City,
    city_pieces::Building::{self},
    content::custom_actions::CustomAction,
    game::Game,
    position::Position,
    resource_pile::ResourcePile,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<(Position, ResourcePile)>,
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct InfluenceCultureAttempt {
    pub starting_city_position: Position,
    pub target_player_index: usize,
    pub target_city_position: Position,
    pub city_piece: Building,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct IncreaseHappiness {
    pub happiness_increases: Vec<(Position, u32)>,
    pub payment: ResourcePile,
}

#[derive(Clone)]
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
    Custom(CustomActionInfo),
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

        self.action_type().is_available(game, player_index)?;

        let p = game.get_player(player_index);

        match self {
            PlayingActionType::Custom(c) => {
                if !p.custom_actions.contains_key(&c.custom_action_type) {
                    return Err("Custom action not available".to_string());
                }

                if c.once_per_turn
                    && p.played_once_per_turn_actions
                        .contains(&c.custom_action_type)
                {
                    return Err("Custom action already played this turn".to_string());
                }
            }
            PlayingActionType::ActionCard(id) => {
                if !p.action_cards.contains(id) {
                    return Err("Action card not available".to_string());
                }

                let civil_card = get_civil_card(*id);
                if !(civil_card.can_play)(game, p) {
                    return Err("Cannot play action card".to_string());
                }
                if let Some(requirement) = civil_card.requirement {
                    if requirement.satisfying_action(game, *id, false).is_none() {
                        return Err("Requirement not met".to_string());
                    }
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
        let _ = p.trigger_event(
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
    pub fn action_type(&self) -> ActionType {
        match self {
            PlayingActionType::Custom(custom_action) => custom_action.action_type.clone(),
            PlayingActionType::ActionCard(id) => get_civil_card(*id).action_type.clone(),
            PlayingActionType::EndTurn => ActionType::cost(ResourcePile::empty()),
            _ => ActionType::regular(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum PlayingAction {
    Advance {
        advance: String,
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
    Custom(CustomAction),
    ActionCard(u8),
    WonderCard(String),
    EndTurn,
}

impl PlayingAction {
    ///
    ///
    /// # Panics
    ///
    /// Panics if action is illegal
    pub fn execute(self, game: &mut Game, player_index: usize, redo: bool) {
        use crate::construct;
        use PlayingAction::*;
        let playing_action_type = self.playing_action_type();
        if !redo {
            let _ = playing_action_type
                .is_available(game, player_index)
                .map_err(|e| panic!("{e}"));
        }
        playing_action_type.action_type().pay(game, player_index);

        match self {
            Advance { advance, payment } => {
                let a = get_advance(&advance);
                assert!(
                    game.get_player(player_index).can_advance(&a),
                    "Illegal action"
                );
                game.get_player(player_index)
                    .advance_cost(&a, Some(&payment))
                    .pay(game, &payment);
                gain_advance(game, &advance, player_index, payment, true);
            }
            FoundCity { settler } => {
                let settler = game.players[player_index].remove_unit(settler);
                assert!(settler.can_found_city(game), "Illegal action");
                build_city(game.get_player_mut(player_index), settler.position);
            }
            Construct(c) => {
                construct::construct(game, player_index, &c);
            }
            Collect(c) => {
                collect(game, player_index, &c);
            }
            Recruit(r) => {
                let player = &mut game.players[player_index];
                if let Some(cost) = recruit_cost(
                    player,
                    &r.units,
                    r.city_position,
                    r.leader_name.as_ref(),
                    &r.replaced_units,
                    Some(&r.payment),
                ) {
                    cost.pay(game, &r.payment);
                } else {
                    panic!("Cannot pay for units")
                }
                recruit(game, player_index, r);
            }
            IncreaseHappiness(i) => {
                increase_happiness(game, player_index, &i.happiness_increases, Some(i.payment));
            }
            InfluenceCultureAttempt(c) => influence_culture_attempt(game, player_index, &c),
            ActionCard(a) => {
                play_action_card(game, player_index, a);
            }
            WonderCard(name) => {
                on_play_wonder_card(
                    game,
                    player_index,
                    WonderCardInfo::new(name, WonderDiscount::default()),
                );
            }
            Custom(custom_action) => {
                custom(game, player_index, custom_action);
            }
            EndTurn => game.next_turn(),
        }
    }

    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        match self {
            PlayingAction::Advance { .. } => PlayingActionType::Advance,
            PlayingAction::FoundCity { .. } => PlayingActionType::FoundCity,
            PlayingAction::Construct(_) => PlayingActionType::Construct,
            PlayingAction::Collect(_) => PlayingActionType::Collect,
            PlayingAction::Recruit(_) => PlayingActionType::Recruit,
            PlayingAction::IncreaseHappiness(_) => PlayingActionType::IncreaseHappiness,
            PlayingAction::InfluenceCultureAttempt(_) => PlayingActionType::InfluenceCultureAttempt,
            PlayingAction::ActionCard(a) => PlayingActionType::ActionCard(*a),
            PlayingAction::WonderCard(name) => PlayingActionType::WonderCard(name.clone()),
            PlayingAction::Custom(c) => PlayingActionType::Custom(c.custom_action_type().info()),
            PlayingAction::EndTurn => PlayingActionType::EndTurn,
        }
    }
}

#[derive(Default, Clone)]
pub struct ActionType {
    pub free: bool,
    pub cost: ResourcePile,
}

impl ActionType {
    pub(crate) fn is_available(&self, game: &Game, player_index: usize) -> Result<(), String> {
        let p = game.get_player(player_index);
        if !p.resources.has_at_least(&self.cost) {
            return Err("Not enough resources for action type".to_string());
        }

        if !(self.free || game.actions_left > 0) {
            return Err("No actions left".to_string());
        }
        Ok(())
    }

    pub(crate) fn pay(&self, game: &mut Game, player_index: usize) {
        let p = game.get_player_mut(player_index);
        let cost = self.cost.clone();
        p.lose_resources(cost.clone());
        if !self.free {
            game.actions_left -= 1;
        }
    }
}

impl ActionType {
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

pub(crate) fn increase_happiness(
    game: &mut Game,
    player_index: usize,
    happiness_increases: &[(Position, u32)],
    payment: Option<ResourcePile>,
) {
    let player = &mut game.players[player_index];
    let mut angry_activations = vec![];
    let mut count = 0;
    for &(city_position, steps) in happiness_increases {
        let city = player.get_city(city_position);
        if steps == 0 {
            continue;
        }

        count += steps * city.size() as u32;

        if city.mood_state == MoodState::Angry {
            angry_activations.push(city_position);
        }
        let city = player.get_city_mut(city_position);
        for _ in 0..steps {
            city.increase_mood_state();
        }
    }

    if let Some(r) = payment {
        player
            .increase_happiness_total_cost(count, Some(&r))
            .pay(game, &r);
    }
}

pub(crate) fn roll_boost_cost(roll: u8) -> ResourcePile {
    ResourcePile::culture_tokens(5 - roll as u32)
}

fn custom(game: &mut Game, player_index: usize, custom_action: CustomAction) {
    let c = custom_action.custom_action_type();
    if c.info().once_per_turn {
        game.players[player_index]
            .played_once_per_turn_actions
            .push(c);
    }
    custom_action.execute(game, player_index);
}

pub(crate) fn build_city(player: &mut Player, position: Position) {
    player.cities.push(City::new(player.index, position));
}
