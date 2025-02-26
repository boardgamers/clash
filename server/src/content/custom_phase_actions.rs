use crate::action::{execute_custom_phase_action, Action};
use crate::advance::undo_advance;
use crate::barbarians::BarbariansEventState;
use crate::content::advances::get_advance;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::undo::UndoContext;
use crate::unit::UnitType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentRequest {
    pub cost: PaymentOptions,
    pub name: String,
    pub optional: bool,
}

impl PaymentRequest {
    #[must_use]
    pub fn new(cost: PaymentOptions, name: String, optional: bool) -> Self {
        Self {
            cost,
            name,
            optional,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ResourceRewardRequest {
    pub reward: PaymentOptions,
    pub name: String,
}

impl ResourceRewardRequest {
    #[must_use]
    pub fn new(reward: PaymentOptions, name: String) -> Self {
        Self { reward, name }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AdvanceRewardRequest {
    pub choices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseRequest {
    Payment(Vec<PaymentRequest>),
    ResourceReward(ResourceRewardRequest),
    AdvanceReward(AdvanceRewardRequest),
    SelectPlayer(PlayerRequest),
    SelectPosition(PositionRequest),
    SelectUnitType(UnitTypeRequest),
    SelectUnits(UnitsRequest),
    BoolRequest,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CustomPhaseEventAction {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    AdvanceReward(String),
    SelectPlayer(usize),
    SelectPosition(Position),
    SelectUnitType(UnitType),
    SelectUnits(Vec<u32>),
    Bool(bool),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentCustomPhaseEvent {
    pub priority: i32,
    pub player_index: usize,
    pub request: CustomPhaseRequest,
    pub response: Option<CustomPhaseEventAction>,
    pub origin: EventOrigin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CustomPhaseEventState {
    pub event_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub players_used: Vec<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_priority_used: Option<i32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<CurrentCustomPhaseEvent>,

    // saved state for other handlers
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barbarians: Option<BarbariansEventState>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_player: Option<usize>,
}

impl CustomPhaseEventState {
    #[must_use]
    pub fn new(event_type: String) -> Self {
        Self {
            players_used: vec![],
            last_priority_used: None,
            current: None,
            barbarians: None,
            selected_player: None,
            event_type,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.players_used.is_empty() && self.last_priority_used.is_none()
    }

    #[must_use]
    pub fn is_active_player(&self) -> bool {
        self.players_used.is_empty()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PositionRequest {
    pub choices: Vec<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PositionRequest {
    #[must_use]
    pub fn new(mut choices: Vec<Position>, description: Option<String>) -> Self {
        choices.sort();
        Self {
            choices,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UnitTypeRequest {
    pub choices: Vec<UnitType>,
    pub player_index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl UnitTypeRequest {
    #[must_use]
    pub fn new(choices: Vec<UnitType>, player_index: usize, description: Option<String>) -> Self {
        Self {
            choices,
            player_index,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UnitsRequest {
    pub player: usize,
    pub choices: Vec<u32>,
    pub needed: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl UnitsRequest {
    #[must_use]
    pub fn new(player: usize, choices: Vec<u32>, needed: u8, description: Option<String>) -> Self {
        Self {
            player,
            choices,
            needed,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PlayerRequest {
    pub choices: Vec<usize>,
    pub description: String,
}

impl PlayerRequest {
    #[must_use]
    pub fn new(choices: Vec<usize>, description: &str) -> Self {
        Self {
            choices,
            description: description.to_string(),
        }
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
                undo_advance(game, &get_advance(&n), player_index, false);
            }
            CustomPhaseEventAction::Bool(_)
            | CustomPhaseEventAction::SelectUnits(_)
            | CustomPhaseEventAction::SelectPlayer(_)
            | CustomPhaseEventAction::SelectPosition(_)
            | CustomPhaseEventAction::SelectUnitType(_) => {
                // done with payer commands - or can't undo
            }
        }
        let Some(UndoContext::CustomPhaseEvent(e)) = game.pop_undo_context() else {
            panic!("when undoing custom phase event, the undo context stack should have a custom phase event")
        };
        game.custom_phase_state.push(*e);
        if let Some(action) = game.action_log.get(game.action_log_index - 1) {
            // is there a better way to do this?
            if let Action::Playing(PlayingAction::Advance { .. }) = action.action {
                game.players[player_index].incident_tokens += 1;
            }
        }
    }

    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = game.current_custom_phase_event_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let event_type = game.current_custom_phase().event_type.clone();
        execute_custom_phase_action(game, player_index, &event_type);
    }
}
