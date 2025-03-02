use crate::action::{execute_custom_phase_action, Action};
use crate::advance::undo_advance;
use crate::barbarians::BarbariansEventState;
use crate::city_pieces::Building;
use crate::content::advances::get_advance;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::undo::UndoContext;
use crate::unit::UnitType;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

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
pub struct AdvanceRequest {
    pub choices: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventRequest {
    Payment(Vec<PaymentRequest>),
    ResourceReward(ResourceRewardRequest),
    SelectAdvance(AdvanceRequest),
    SelectPlayer(PlayerRequest),
    SelectPosition(PositionRequest),
    SelectUnitType(UnitTypeRequest),
    SelectUnits(UnitsRequest),
    SelectStructures(StructuresRequest),
    BoolRequest,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventResponse {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    SelectAdvance(String),
    SelectPlayer(usize),
    SelectPosition(Position),
    SelectUnitType(UnitType),
    SelectUnits(Vec<u32>),
    SelectStructures(Vec<(Position, Structure)>),
    Bool(bool),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentEventHandler {
    pub priority: i32,
    pub request: CurrentEventRequest,
    pub response: Option<CurrentEventResponse>,
    pub origin: EventOrigin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentEventPlayer {
    #[serde(rename = "player")]
    pub index: usize,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_priority_used: Option<i32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<CurrentEventHandler>,

    // saved state for other handlers
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub payment: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub must_reduce_mood: Vec<Position>,
}

impl CurrentEventPlayer {
    #[must_use]
    pub fn new(current_player: usize) -> Self {
        Self {
            index: current_player,
            last_priority_used: None,
            handler: None,
            payment: ResourcePile::empty(),
            must_reduce_mood: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentEvent {
    pub event_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub players_used: Vec<usize>,

    #[serde(flatten)]
    pub player: CurrentEventPlayer,

    // saved state for other handlers
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barbarians: Option<BarbariansEventState>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_player: Option<usize>,
}

impl CurrentEvent {
    #[must_use]
    pub fn new(event_type: String, current_player: usize) -> Self {
        Self {
            players_used: vec![],
            player: CurrentEventPlayer::new(current_player),
            barbarians: None,
            selected_player: None,
            event_type,
        }
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
pub enum Structure {
    CityCenter,
    Building(Building),
    Wonder(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct StructuresRequest {
    pub choices: Vec<(Position, Structure)>,
    pub needed: RangeInclusive<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl StructuresRequest {
    #[must_use]
    pub fn new(
        choices: Vec<(Position, Structure)>,
        needed: RangeInclusive<u8>,
        description: Option<String>,
    ) -> Self {
        Self {
            choices,
            needed,
            description,
        }
    }

    #[must_use]
    pub fn is_valid(&self, game: &Game, selected: &[(Position, Structure)]) -> bool {
        self.needed.contains(&(selected.len() as u8)) && Self::city_center_last(game, selected)
    }

    fn city_center_last(game: &Game, selected: &[(Position, Structure)]) -> bool {
        let x = selected
            .iter()
            .chunk_by(|(p, _s)| p)
            .into_iter()
            .all(|(&p, g)| {
                let v = g.collect_vec();
                let x1 = v.len() == game.get_any_city(p).expect("city should exist").size()
                    || !v.iter().any(|(_p, s)| matches!(s, &Structure::CityCenter));
                x1
            });
        x
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

impl CurrentEventResponse {
    pub(crate) fn undo(self, game: &mut Game, player_index: usize) {
        match self {
            CurrentEventResponse::Payment(p) => {
                let player = &mut game.players[player_index];
                for p in p {
                    player.gain_resources_in_undo(p);
                }
            }
            CurrentEventResponse::ResourceReward(r) => {
                game.players[player_index].lose_resources(r);
            }
            CurrentEventResponse::SelectAdvance(n) => {
                undo_advance(game, &get_advance(&n), player_index, false);
            }
            CurrentEventResponse::Bool(_)
            | CurrentEventResponse::SelectUnits(_)
            | CurrentEventResponse::SelectPlayer(_)
            | CurrentEventResponse::SelectPosition(_)
            | CurrentEventResponse::SelectStructures(_)
            | CurrentEventResponse::SelectUnitType(_) => {
                // done with payer commands - or can't undo
            }
        }
        let Some(UndoContext::CustomPhaseEvent(e)) = game.pop_undo_context() else {
            panic!("when undoing custom phase event, the undo context stack should have a custom phase event")
        };
        game.current_events.push(*e);
        if let Some(action) = game.action_log.get(game.action_log_index - 1) {
            // is there a better way to do this?
            if let Action::Playing(PlayingAction::Advance { .. }) = action.action {
                game.players[player_index].incident_tokens += 1;
            }
        }
    }

    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = game.current_event_handler_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let event_type = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, &event_type);
    }
}
