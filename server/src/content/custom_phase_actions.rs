use crate::ability_initializer::EventOrigin;
use crate::action::Action;
use crate::content::advances::get_advance;
use crate::game::{Game, UndoContext};
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingAction;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventType {
    StartCombatAttacker,
    StartCombatDefender,
    TurnStart,
    OnAdvance,
    OnConstruct,
}

impl CustomPhaseEventType {
    #[must_use]
    pub fn is_last_type_for_event(&self) -> bool {
        !matches!(self, CustomPhaseEventType::StartCombatAttacker)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhasePaymentRequest {
    pub cost: PaymentOptions,
    pub name: String,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhaseResourceRewardRequest {
    pub reward: PaymentOptions,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhaseAdvanceRewardRequest {
    pub choices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseRequest {
    Payment(Vec<CustomPhasePaymentRequest>),
    ResourceReward(CustomPhaseResourceRewardRequest),
    AdvanceReward(CustomPhaseAdvanceRewardRequest),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventAction {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    AdvanceReward(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentCustomPhaseEvent {
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
    pub current: Option<CurrentCustomPhaseEvent>,
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

impl CustomPhaseEventAction {
    pub(crate) fn undo(self, game: &mut Game, player_index: usize) {
        match self {
            CustomPhaseEventAction::Payment(p) => {
                let player = &mut game.players[player_index];
                for p in p {
                    player.gain_resources_in_undo(p);
                }
            }
            CustomPhaseEventAction::ResourceReward(r) => {
                game.players[player_index].lose_resources(r);
            }
            CustomPhaseEventAction::AdvanceReward(n) => {
                game.undo_advance(&get_advance(&n), player_index, false);
            }
        }
        let Some(UndoContext::CustomPhaseEvent(e)) = game.pop_undo_context() else {
            panic!("when undoing custom phase event, the undo context stack should have a custom phase event")
        };
        game.custom_phase_state = e;
        if let Some(action) = game.action_log.get(game.action_log_index - 1) {
            // is there a better way to do this?
            if let Action::Playing(PlayingAction::Advance { .. }) = action.action {
                game.players[player_index].game_event_tokens += 1;
            }
        }
    }

    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = &mut game.custom_phase_state.current else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let event_type = s.event_type.clone();
        game.execute_custom_phase_action(player_index, &event_type);
    }
}
