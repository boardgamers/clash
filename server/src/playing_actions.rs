use crate::action_cost::ActionResourceCost;
use serde::{Deserialize, Serialize};

use crate::ability_initializer::AbilityInitializerSetup;
use crate::action::pay_action;
use crate::action_card::{can_play_civil_card, discard_action_card, play_action_card};
use crate::action_cost::ActionCost;
use crate::advance::{AdvanceAction, execute_advance_action};
use crate::card::HandCardLocation;
use crate::city::execute_found_city_action;
use crate::collect::{Collect, base_collect_event_origin, execute_collect};
use crate::construct::Construct;
use crate::content::ability::{
    Ability, advance_event_origin, construct_event_origin, recruit_event_origin,
};
use crate::content::custom_actions::{
    CustomAction, CustomActionActivation, PlayingActionModifier, SpecialAction,
    can_play_special_action, on_custom_action,
};
use crate::content::persistent_events::{
    PaymentRequest, PersistentEventType, TriggerPersistentEventParams, trigger_persistent_event_ext,
};
use crate::cultural_influence::{
    InfluenceCultureAttempt, execute_influence_culture_attempt, influence_base_origin,
};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::GameState;
use crate::happiness::{
    IncreaseHappiness, execute_increase_happiness, happiness_base_event_origin,
    happiness_event_origin,
};
use crate::log::{ActionLogBalance, ActionLogEntry};
use crate::movement::move_event_origin;
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
    Special(SpecialAction),
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
            PlayingActionType::Special(c) => {
                can_play_special_action(game, p, *c)?;
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
            PlayingActionType::Special(custom_action) => game
                .player(player)
                .special_action_info(custom_action)
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
            cost.payment_options(p, self.origin(p))
        }
    }

    pub(crate) fn origin(&self, player: &Player) -> EventOrigin {
        match self {
            PlayingActionType::Advance => advance_event_origin(),
            PlayingActionType::FoundCity => EventOrigin::Ability("Found City".to_string()),
            PlayingActionType::Construct => construct_event_origin(),
            PlayingActionType::Collect => base_collect_event_origin(),
            PlayingActionType::Recruit => recruit_event_origin(),
            PlayingActionType::IncreaseHappiness => happiness_base_event_origin(),
            PlayingActionType::InfluenceCultureAttempt => influence_base_origin(),
            PlayingActionType::ActionCard(a) => EventOrigin::CivilCard(*a),
            PlayingActionType::WonderCard(_) => wonder_origin(),
            PlayingActionType::Special(c) => player.special_action_info(c).event_origin,
            PlayingActionType::MoveUnits => move_event_origin(),
            PlayingActionType::EndTurn => end_turn_origin(),
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
        let p = game.player(player_index);
        let playing_action_type = self.playing_action_type(p);
        if !redo {
            playing_action_type.is_available(game, player_index)?;
        }
        let action_cost = playing_action_type.cost(game, player_index);
        if !action_cost.free {
            pay_action(
                game,
                &EventPlayer::new(player_index, playing_action_type.origin(p)),
            );
        }

        self.execute_without_action_cost(game, player_index)
    }

    pub(crate) fn execute_without_action_cost(
        self,
        game: &mut Game,
        player_index: usize,
    ) -> Result<(), String> {
        // log these before the payment for clarity
        if let PlayingAction::ActionCard(id) = &self {
            discard_action_card(
                game,
                player_index,
                *id,
                &EventOrigin::Ability("Action Card".to_string()),
                HandCardLocation::PlayToDiscard,
            );
        }

        let action_type = self.playing_action_type(game.player(player_index));
        let override_origin = add_override_origin(game, player_index, &action_type);
        ActionPayment::new(self).on_pay_action(game, player_index, override_origin)
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
                        wonder_origin(),
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
            PlayingAction::Custom(c) => PlayingActionType::Special(SpecialAction::Custom(c.action)),
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
        PlayingActionType::Special(SpecialAction::Modifier(c)) => {
            assert!(player.custom_action_modifiers(base_type).contains(c));
        }
        _ => {
            assert!(base_type == playing_action_type);
        }
    }
    playing_action_type.clone()
}

#[must_use]
pub(crate) fn base_or_modified_available(
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
                .map(PlayingActionModifier::playing_action_type),
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
    if game.actions_left > 0 {
        EventPlayer::new(player, end_turn_origin()).add_log_entry(
            game,
            ActionLogEntry::action(ActionLogBalance::Loss, game.actions_left),
        );
    }
    game.next_turn();
}

pub(crate) fn end_turn_origin() -> EventOrigin {
    EventOrigin::Ability("End Turn".to_string())
}

pub(crate) fn wonder_origin() -> EventOrigin {
    EventOrigin::Ability("Build Wonder".to_string())
}

fn add_override_origin(
    game: &mut Game,
    player_index: usize,
    action_type: &PlayingActionType,
) -> Option<EventOrigin> {
    match action_type {
        PlayingActionType::Special(c) => {
            if let Some(key) = &game
                .player(player_index)
                .special_action_info(c)
                .cost
                .once_per_turn
            {
                game.players[player_index]
                    .played_once_per_turn_actions
                    .push(*key);
            }
            Some(
                game.player(player_index)
                    .special_action_info(c)
                    .event_origin,
            )
        }
        PlayingActionType::ActionCard(c) => Some(EventOrigin::CivilCard(*c)),
        _ => None,
    }
}
