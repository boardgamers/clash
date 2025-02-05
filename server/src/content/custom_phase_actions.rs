use crate::ability_initializer::EventOrigin;
use crate::game::{Game, UndoContext};
use crate::payment::PaymentOptions;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventType {
    StartCombatAttacker,
    StartCombatDefender,
    TurnStart,
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
pub struct CustomPhaseRewardRequest {
    pub reward: PaymentOptions,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseRequest {
    Payment(Vec<CustomPhasePaymentRequest>),
    Reward(CustomPhaseRewardRequest),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventAction {
    Payment(Vec<ResourcePile>),
    Reward(ResourcePile),
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
            CustomPhaseEventAction::Payment(_) => todo!("undo payment"),
            CustomPhaseEventAction::Reward(r) => {
                let player = &mut game.players[player_index];
                player.loose_resources(r);
            }
        }
        let Some(UndoContext::CustomPhaseEvent(e)) = game.pop_undo_context() else {
            panic!("when undoing custom phase event, the undo context stack should have a custom phase event")
        };
        game.custom_phase_state = e;
    }

    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(c) = &mut game.custom_phase_state.current else {
            panic!("current custom phase event should be set")
        };
        let c = c.clone();

        let events = game.players[player_index]
            .events
            .take()
            .expect("events should be set");
        events.redo_custom_phase_action.trigger(game, &c, &self);
        game.players[player_index].events = Some(events);
    }
}
