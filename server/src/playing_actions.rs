use crate::action_cost::ActionResourceCost;
use serde::{Deserialize, Serialize};

use crate::ability_initializer::AbilityInitializerSetup;
use crate::action_card::{can_play_civil_card, discard_action_card, play_action_card};
use crate::action_cost::ActionCost;
use crate::advance::{AdvanceAction, execute_advance_action};
use crate::card::HandCardLocation;
use crate::city::execute_found_city_action;
use crate::collect::{Collect, execute_collect};
use crate::construct::Construct;
use crate::content::ability::Ability;
use crate::content::custom_actions::{
    CustomAction, CustomActionActivation, CustomActionType, can_play_custom_action,
    log_start_custom_action, on_custom_action,
};
use crate::content::persistent_events::{
    PaymentRequest, PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_ext,
};
use crate::cultural_influence::{InfluenceCultureAttempt, execute_influence_culture_attempt};
use crate::events::EventOrigin;
use crate::game::GameState;
use crate::happiness::{IncreaseHappiness, execute_increase_happiness, happiness_event_origin};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::recruit::{Recruit, execute_recruit};
use crate::wonder::{Wonder, WonderCardInfo, cities_for_wonder, on_play_wonder_card, wonder_cost};
use crate::{game::Game, resource_pile::ResourcePile};

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

        self.cost(game, player_index)
            .is_available(game, player_index)?;

        let p = game.player(player_index);

        match self {
            PlayingActionType::Custom(c) => {
                can_play_custom_action(game, p, *c)?;
            }
            PlayingActionType::ActionCard(id) => {
                can_play_civil_card(game, p, *id)?;
            }
            PlayingActionType::WonderCard(w) => {
                if !p.wonder_cards.contains(w) {
                    return Err("Wonder card not available".to_string());
                }

                if cities_for_wonder(*w, game, p, wonder_cost(game, p, *w)).is_empty() {
                    return Err("no cities for wonder".to_string());
                }
            }
            _ => {}
        }

        let mut possible: Result<(), String> = Ok(());
        p.trigger_event(
            |e| &e.is_playing_action_available,
            &mut possible,
            game,
            self,
        );
        possible
    }

    #[must_use]
    pub fn cost(&self, game: &Game, player: usize) -> ActionCost {
        match self {
            PlayingActionType::Custom(custom_action) => game
                .player(player)
                .custom_action_info(*custom_action)
                .cost
                .cost
                .clone(),
            PlayingActionType::ActionCard(id) => game.cache.get_civil_card(*id).action_type.clone(),
            // action cost of wonder is checked later
            PlayingActionType::WonderCard(_) | PlayingActionType::EndTurn => {
                ActionCost::new(true, ActionResourceCost::free())
            }
            _ => ActionCost::new(false, ActionResourceCost::free()),
        }
    }

    #[must_use]
    pub fn payment_options(&self, game: &Game, player_index: usize) -> PaymentOptions {
        let p = game.player(player_index);
        let cost = self.cost(game, player_index);
        if let ActionResourceCost::Free = cost.cost {
            PaymentOptions::free()
        } else {
            cost.payment_options(p, self.event_origin(p))
        }
    }

    pub(crate) fn event_origin(&self, player: &Player) -> EventOrigin {
        match self {
            PlayingActionType::Custom(c) => player.custom_action_info(*c).event_origin,
            PlayingActionType::ActionCard(id) => EventOrigin::CivilCard(*id),
            _ => panic!(
                "PlayingAction::payable_action_name called on an action \
                that is not payable up front: {self:?}",
            ),
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
        let playing_action_type = self.playing_action_type(game.player(player_index));
        if !redo {
            playing_action_type.is_available(game, player_index)?;
        }
        let action_cost = playing_action_type.cost(game, player_index);
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
        // log these before the payment for clarity
        match &self {
            PlayingAction::Custom(a) => {
                log_start_custom_action(game, player_index, a);
            }
            PlayingAction::ActionCard(id) => {
                discard_action_card(
                    game,
                    player_index,
                    *id,
                    &EventOrigin::Ability("Action Card".to_string()),
                    HandCardLocation::PlayToDiscard,
                );
            }
            _ => {}
        }

        let action_type = self.playing_action_type(game.player(player_index));
        let origin_override = match action_type {
            PlayingActionType::Custom(c) => {
                if let Some(key) = &game
                    .player(player_index)
                    .custom_action_info(c)
                    .cost
                    .once_per_turn
                {
                    game.players[player_index]
                        .played_once_per_turn_actions
                        .push(*key);
                }
                Some(game.player(player_index).custom_action_info(c).event_origin)
            }
            PlayingActionType::ActionCard(c) => Some(EventOrigin::CivilCard(c)),
            _ => None,
        };

        let payment_options = action_type.payment_options(game, player_index);
        if !payment_options.is_free() {
            game.add_info_log_item(&format!(
                "{} has to pay {} for {}",
                game.player_name(player_index),
                payment_options.default,
                action_type
                    .event_origin(game.player(player_index))
                    .name(game)
            ));
        }

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
            Advance(a) => execute_advance_action(game, player_index, &a)?,
            FoundCity { settler } => execute_found_city_action(game, player_index, settler)?,
            Construct(c) => construct::execute_construct(game, player_index, &c)?,
            Collect(c) => execute_collect(game, player_index, &c)?,
            Recruit(r) => execute_recruit(game, player_index, r)?,
            IncreaseHappiness(i) => execute_increase_happiness(
                game,
                player_index,
                &i.happiness_increases,
                &i.payment,
                false,
                &i.action_type,
                &happiness_event_origin(&i.action_type, game.player(player_index)),
            )?,
            InfluenceCultureAttempt(c) => {
                execute_influence_culture_attempt(game, player_index, &c)?;
            }
            ActionCard(a) => play_action_card(game, player_index, a),
            WonderCard(w) => {
                on_play_wonder_card(
                    game,
                    player_index,
                    WonderCardInfo::new(
                        w,
                        wonder_cost(game, game.player(player_index), w),
                        EventOrigin::Ability("Build Wonder".to_string()),
                    ),
                );
            }
            Custom(custom_action) => {
                on_custom_action(
                    game,
                    player_index,
                    CustomActionActivation::new(custom_action, action_payment),
                );
            }
            EndTurn => {
                end_turn(game, player_index);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn playing_action_type(&self, player: &Player) -> PlayingActionType {
        match self {
            PlayingAction::Advance(_) => PlayingActionType::Advance,
            PlayingAction::FoundCity { .. } => PlayingActionType::FoundCity,
            PlayingAction::Construct(_) => PlayingActionType::Construct,
            PlayingAction::Collect(c) => {
                assert_allowed_action_type(&c.action_type, &PlayingActionType::Collect, player)
            }
            PlayingAction::Recruit(_) => PlayingActionType::Recruit,
            PlayingAction::IncreaseHappiness(h) => assert_allowed_action_type(
                &h.action_type,
                &PlayingActionType::IncreaseHappiness,
                player,
            ),
            PlayingAction::InfluenceCultureAttempt(i) => assert_allowed_action_type(
                &i.action_type,
                &PlayingActionType::InfluenceCultureAttempt,
                player,
            ),
            PlayingAction::ActionCard(a) => PlayingActionType::ActionCard(*a),
            PlayingAction::WonderCard(name) => PlayingActionType::WonderCard(*name),
            PlayingAction::Custom(c) => PlayingActionType::Custom(c.action),
            PlayingAction::EndTurn => PlayingActionType::EndTurn,
        }
    }
}

fn assert_allowed_action_type(
    playing_action_type: &PlayingActionType,
    base_type: &PlayingActionType,
    player: &Player,
) -> PlayingActionType {
    match playing_action_type {
        PlayingActionType::Custom(c) => {
            assert!(player.custom_action_modifiers(base_type).contains(c));
        }
        _ => {
            assert!(base_type == playing_action_type);
        }
    }
    playing_action_type.clone()
}

#[must_use]
pub(crate) fn base_or_custom_available(
    game: &Game,
    player: usize,
    base: &PlayingActionType,
) -> Vec<PlayingActionType> {
    vec![base.clone()]
        .into_iter()
        .chain(
            game.player(player)
                .custom_action_modifiers(base)
                .iter()
                .map(CustomActionType::playing_action_type),
        )
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

pub(crate) fn pay_for_action() -> Ability {
    Ability::builder(
        "Pay for action",
        "origin is overridden - so this text is not shown",
    )
    .add_payment_request_listener(
        |e| &mut e.pay_action,
        0,
        |game, p, a| {
            if matches!(a.action, PlayingAction::IncreaseHappiness(_)) {
                // handled in the happiness action
                return None;
            }

            let payment_options = a
                .action
                .playing_action_type(game.player(p.index))
                .payment_options(game, p.index);
            if payment_options.is_free() {
                return None;
            }

            Some(vec![PaymentRequest::mandatory(
                payment_options,
                "Pay for action",
            )])
        },
        |_game, _s, _a| {},
    )
    .build()
}

fn end_turn(game: &mut Game, player: usize) {
    game.add_info_log_item(&format!(
        "{} ended their turn{}",
        game.player(player),
        match game.actions_left {
            0 => String::new(),
            actions_left => format!(" with {actions_left} actions left"),
        }
    ));
    game.next_turn();
}
