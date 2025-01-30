use crate::ability_initializer::EventOrigin;
use crate::consts::ACTIONS;
use crate::content::trade_routes::{gain_trade_route_reward, trade_route_reward};
use crate::game::{Game, GameState};
use crate::payment::PaymentModel;
use crate::player::Player;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

//todo remove
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseState {
    TradeRouteSelection,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventType {
    StartCombatAttacker,
    StartCombatDefender,
}

impl CustomPhaseEventType {
    #[must_use]
    pub fn is_last_type_for_event(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            CustomPhaseEventType::StartCombatAttacker => false,
            _ => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhasePaymentRequest {
    pub model: PaymentModel,
    pub name: String,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseRequest {
    Payment(Vec<CustomPhasePaymentRequest>),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventAction {
    Payment(Vec<ResourcePile>),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhaseEvent {
    pub event_type: CustomPhaseEventType,
    pub priority: i32,
    pub player_index: usize,
    pub request: CustomPhaseRequest,
    pub response: Option<CustomPhaseEventAction>,
    pub origin: EventOrigin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct CustomPhaseEventState {
    pub event_used: Vec<CustomPhaseEventType>,
    pub last_priority_used: Option<i32>,
    pub current: Option<CustomPhaseEvent>,
}

impl CustomPhaseEventState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            event_used: vec![],
            last_priority_used: None,
            current: None,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.event_used.is_empty() && self.last_priority_used.is_none()
    }
}

//todo remove this
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum CustomPhaseAction {
    TradeRouteSelectionAction(ResourcePile),
}

impl CustomPhaseAction {
    ///
    /// # Panics
    /// Panics if the action cannot be executed
    pub fn execute(self, game: &mut Game, player_index: usize) {
        match &game.state {
            GameState::CustomPhase(state) => match state {
                CustomPhaseState::TradeRouteSelection => {
                    if let CustomPhaseAction::TradeRouteSelectionAction(p) = self {
                        let (reward, routes) =
                            trade_route_reward(game).expect("No trade route reward");
                        assert!(reward.is_valid_payment(&p), "Invalid payment"); // it's a gain
                        gain_trade_route_reward(game, player_index, &routes, &p);
                        game.state = GameState::Playing;
                    } else {
                        panic!("Need to pass TradeRouteSelectionAction to execute");
                    }
                }
            },
            _ => panic!("can only execute custom phase actions if the game is in a custom phase"),
        }
    }

    ///
    /// # Panics
    /// Panics if the action cannot be undone
    pub fn undo(self, game: &mut Game, _player_index: usize) {
        match self {
            CustomPhaseAction::TradeRouteSelectionAction(p) => {
                match game.state {
                    GameState::Playing if game.actions_left == ACTIONS => {}
                    _ => {
                        panic!(
                            "can only undo trade route selection if the game is in playing state"
                        );
                    }
                }
                game.players[game.current_player_index].loose_resources(p);
                game.state = GameState::CustomPhase(CustomPhaseState::TradeRouteSelection);
            }
        }
    }

    #[must_use]
    pub fn format_log_item(&self, _game: &Game, _player: &Player, player_name: &str) -> String {
        match self {
            CustomPhaseAction::TradeRouteSelectionAction(_) => {
                format!("{player_name} selected trade routes",)
            }
        }
    }
}
