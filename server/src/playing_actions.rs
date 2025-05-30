use serde::{Deserialize, Serialize};

use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{can_play_civil_card, play_action_card};
use crate::advance::{AdvanceAction, base_advance_cost, gain_advance_without_payment};
use crate::city::found_city;
use crate::collect::{PositionCollection, collect};
use crate::construct::Construct;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::{
    CustomAction, CustomActionActivation, CustomActionType, can_play_custom_action,
    collect_modifiers, execute_custom_action, happiness_modifiers, influence_modifiers,
};
use crate::content::persistent_events::{
    PaymentRequest, PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_ext,
};
use crate::cultural_influence::{InfluenceCultureAttempt, influence_culture_attempt};
use crate::events::EventOrigin;
use crate::game::GameState;
use crate::happiness::increase_happiness;
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player::{Player, remove_unit};
use crate::player_events::PlayingActionInfo;
use crate::recruit::recruit;
use crate::unit::{UnitType, Units};
use crate::wonder::{
    Wonder, WonderCardInfo, WonderDiscount, cities_for_wonder, on_play_wonder_card,
};
use crate::{game::Game, position::Position, resource_pile::ResourcePile};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<PositionCollection>,
    pub action_type: PlayingActionType,
}

impl Collect {
    #[must_use]
    pub fn new(
        city_position: Position,
        collections: Vec<PositionCollection>,
        action_type: PlayingActionType,
    ) -> Self {
        Self {
            city_position,
            collections,
            action_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Recruit {
    pub units: Vec<UnitType>,
    pub city_position: Position,
    pub payment: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaced_units: Vec<u32>,
}

impl Recruit {
    #[must_use]
    pub fn new(units: Vec<UnitType>, city_position: Position, payment: ResourcePile) -> Self {
        Self {
            units,
            city_position,
            payment,
            replaced_units: Vec::new(),
        }
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
    WonderCard(Wonder),
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
                can_play_custom_action(game, p, *c)?;
            }
            PlayingActionType::ActionCard(id) => {
                can_play_civil_card(game, p, *id)?;
            }
            PlayingActionType::WonderCard(name) => {
                if !p.wonder_cards.contains(name) {
                    return Err("Wonder card not available".to_string());
                }

                if cities_for_wonder(*name, game, p, &WonderDiscount::default()).is_empty() {
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
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub enum PlayingAction {
    Advance(AdvanceAction),
    FoundCity { settler: u32 },
    Construct(Construct),
    Collect(Collect),
    Recruit(Recruit),
    IncreaseHappiness(IncreaseHappiness),
    InfluenceCultureAttempt(InfluenceCultureAttempt),
    Custom(CustomAction),
    ActionCard(u8),
    WonderCard(Wonder),
    EndTurn,
}

impl PlayingAction {
    pub(crate) fn execute(
        self,
        game: &mut Game,
        player_index: usize,
        redo: bool,
    ) -> Result<(), String> {
        let playing_action_type = self.playing_action_type();
        if !redo {
            playing_action_type.is_available(game, player_index)?;
        }
        let action_cost = playing_action_type.cost(game);
        if !action_cost.free {
            game.actions_left -= 1;
        }

        self.execute_without_action_cost(game, player_index)
    }

    pub(crate) fn execute_without_action_cost(
        self,
        game: &mut Game,
        player_index: usize,
    ) -> Result<(), String> {
        let origin_override = match self.playing_action_type() {
            PlayingActionType::Custom(c) => {
                if let Some(key) = c.info().once_per_turn {
                    game.players[player_index]
                        .played_once_per_turn_actions
                        .push(key);
                }
                Some(game.player(player_index).custom_action_origin(&c))
            }
            PlayingActionType::ActionCard(c) => Some(EventOrigin::CivilCard(c)),
            _ => None,
        };

        ActionPayment::new(self).on_pay_action(game, player_index, origin_override)
    }

    pub(crate) fn execute_without_cost(
        self,
        game: &mut Game,
        player_index: usize,
        action_payment: ResourcePile,
    ) -> Result<(), String> {
        use crate::construct;
        use PlayingAction::*;
        match self {
            Advance(a) => {
                let advance = a.advance;
                if !game.player(player_index).can_advance(advance, game) {
                    return Err("Cannot advance".to_string());
                }
                game.player(player_index)
                    .advance_cost(advance, game, game.execute_cost_trigger())
                    .pay(game, &a.payment);
                gain_advance_without_payment(game, advance, player_index, a.payment, true);
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
            IncreaseHappiness(i) => increase_happiness(
                game,
                player_index,
                &i.happiness_increases,
                Some(i.payment),
                &i.action_type,
            )?,
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
                execute_custom_action(
                    game,
                    player_index,
                    CustomActionActivation::new(custom_action, action_payment),
                );
            }
            EndTurn => game.next_turn(),
        }
        Ok(())
    }

    #[must_use]
    pub fn playing_action_type(&self) -> PlayingActionType {
        match self {
            PlayingAction::Advance(_) => PlayingActionType::Advance,
            PlayingAction::FoundCity { .. } => PlayingActionType::FoundCity,
            PlayingAction::Construct(_) => PlayingActionType::Construct,
            PlayingAction::Collect(c) => assert_allowed(
                &c.action_type,
                &PlayingActionType::Collect,
                &collect_modifiers(),
            ),
            PlayingAction::Recruit(_) => PlayingActionType::Recruit,
            PlayingAction::IncreaseHappiness(h) => assert_allowed(
                &h.action_type,
                &PlayingActionType::IncreaseHappiness,
                &happiness_modifiers(),
            ),
            PlayingAction::InfluenceCultureAttempt(i) => assert_allowed(
                &i.action_type,
                &PlayingActionType::InfluenceCultureAttempt,
                &influence_modifiers(),
            ),
            PlayingAction::ActionCard(a) => PlayingActionType::ActionCard(*a),
            PlayingAction::WonderCard(name) => PlayingActionType::WonderCard(*name),
            PlayingAction::Custom(c) => PlayingActionType::Custom(c.action),
            PlayingAction::EndTurn => PlayingActionType::EndTurn,
        }
    }
}

fn assert_allowed(
    playing_action_type: &PlayingActionType,
    base_type: &PlayingActionType,
    allowed_modifiers: &[CustomActionType],
) -> PlayingActionType {
    match playing_action_type {
        PlayingActionType::Custom(c) => {
            assert!(allowed_modifiers.contains(c));
        }
        _ => {
            assert!(base_type == playing_action_type);
        }
    }
    playing_action_type.clone()
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionResourceCost {
    Resources(ResourcePile),
    AdvanceCostWithoutDiscount,
}

impl ActionResourceCost {
    #[must_use]
    pub fn free() -> Self {
        ActionResourceCost::Resources(ResourcePile::empty())
    }

    #[must_use]
    pub fn resources(cost: ResourcePile) -> Self {
        ActionResourceCost::Resources(cost)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionCost {
    pub free: bool,
    pub cost: ActionResourceCost,
}

impl ActionCost {
    pub(crate) fn is_available(&self, game: &Game, player_index: usize) -> Result<(), String> {
        let p = game.player(player_index);
        if !p.can_afford(&self.payment_options(p)) {
            return Err("Not enough resources for action type".to_string());
        }

        if !(self.free || game.actions_left > 0) {
            return Err("No actions left".to_string());
        }
        Ok(())
    }

    #[must_use]
    pub fn payment_options(&self, player: &Player) -> PaymentOptions {
        match &self.cost {
            ActionResourceCost::Resources(c) => {
                PaymentOptions::resources(player, PaymentReason::ActionCard, c.clone())
            }
            ActionResourceCost::AdvanceCostWithoutDiscount => base_advance_cost(player),
        }
    }
}

impl ActionCost {
    #[must_use]
    pub fn cost(cost: ResourcePile) -> Self {
        Self::new(true, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub fn regular() -> Self {
        Self::new(false, ActionResourceCost::free())
    }

    #[must_use]
    pub fn regular_with_cost(cost: ResourcePile) -> Self {
        Self::new(false, ActionResourceCost::resources(cost))
    }

    #[must_use]
    pub fn free() -> Self {
        Self::new(true, ActionResourceCost::free())
    }

    #[must_use]
    pub fn new(free: bool, cost: ActionResourceCost) -> Self {
        Self { free, cost }
    }
}

#[must_use]
pub(crate) fn base_or_custom_available(
    game: &Game,
    player: usize,
    action: PlayingActionType,
    custom: Vec<CustomActionType>,
) -> Vec<PlayingActionType> {
    vec![action]
        .into_iter()
        .chain(custom.into_iter().map(|c| c.playing_action_type()))
        .filter_map(|a| a.is_available(game, player).map(|()| a).ok())
        .collect()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ActionPayment {
    pub action: PlayingAction,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub payment: ResourcePile,
}

impl ActionPayment {
    #[must_use]
    pub fn new(action: PlayingAction) -> Self {
        Self {
            action,
            payment: ResourcePile::empty(),
        }
    }

    pub(crate) fn on_pay_action(
        self,
        game: &mut Game,
        player_index: usize,
        origin_override: Option<EventOrigin>,
    ) -> Result<(), String> {
        let Some(a) = trigger_persistent_event_ext(
            game,
            &[player_index],
            |e| &mut e.pay_action,
            self,
            PersistentEventType::PayAction,
            TriggerPersistentEventParams {
                origin_override,
                ..Default::default()
            },
        ) else {
            return Ok(());
        };

        a.action.execute_without_cost(game, player_index, a.payment)
    }
}

pub(crate) fn pay_for_action() -> Builtin {
    Builtin::builder("Pay for action card", "")
        .add_payment_request_listener(
            |e| &mut e.pay_action,
            0,
            |game, player_index, a| {
                if matches!(a.action, PlayingAction::IncreaseHappiness(_)) {
                    // handled in the happiness action
                    return None;
                }

                let payment_options = a
                    .action
                    .playing_action_type()
                    .cost(game)
                    .payment_options(game.player(player_index));
                if payment_options.is_free() {
                    return None;
                }

                Some(vec![PaymentRequest::mandatory(
                    payment_options,
                    "Pay for action",
                )])
            },
            |game, s, a| {
                a.payment = s.choice[0].clone();
                game.add_info_log_item(&format!(
                    "{} paid {} for the action",
                    s.player_name, s.choice[0]
                ));
            },
        )
        .build()
}
