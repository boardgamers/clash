use crate::action::execute_custom_phase_action;
use crate::barbarians::BarbariansEventState;
use crate::city_pieces::Building;
use crate::combat::Combat;
use crate::combat_listeners::{CombatEnd, CombatRoundEnd};
use crate::events::EventOrigin;
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::map::Rotation;
use crate::payment::PaymentOptions;
use crate::player_events::{AdvanceInfo, IncidentInfo};
use crate::playing_actions::Recruit;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernmentType, StatusPhaseState};
use crate::unit::UnitType;
use itertools::Itertools;
use num::Zero;
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
    pub fn new(cost: PaymentOptions, name: &str, optional: bool) -> Self {
        Self {
            cost,
            name: name.to_string(),
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

impl AdvanceRequest {
    #[must_use]
    pub fn new(choices: Vec<String>) -> Self {
        Self { choices }
    }
}

pub type SelectedStructure = (Position, Structure);

pub type StructuresRequest = MultiRequest<SelectedStructure>;

#[must_use]
pub fn is_selected_structures_valid(game: &Game, selected: &[SelectedStructure]) -> bool {
    selected
        .iter()
        .chunk_by(|(p, _s)| p)
        .into_iter()
        .all(|(&p, g)| {
            let v = g.collect_vec();
            v.len() == game.get_any_city(p).size()
                || !v.iter().any(|(_p, s)| matches!(s, &Structure::CityCenter))
        })
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventRequest {
    Payment(Vec<PaymentRequest>),
    ResourceReward(ResourceRewardRequest),
    SelectAdvance(AdvanceRequest),
    SelectPlayer(PlayerRequest),
    SelectPositions(PositionRequest),
    SelectUnitType(UnitTypeRequest),
    SelectUnits(UnitsRequest),
    SelectStructures(StructuresRequest),
    BoolRequest(String),
    ChangeGovernment(ChangeGovernmentRequest),
    ExploreResolution,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventResponse {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    SelectAdvance(String),
    SelectPlayer(usize),
    SelectPositions(Vec<Position>),
    SelectUnitType(UnitType),
    SelectUnits(Vec<u32>),
    SelectStructures(Vec<SelectedStructure>),
    Bool(bool),
    ChangeGovernmentType(ChangeGovernmentType),
    ExploreResolution(Rotation),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentEventHandler {
    pub priority: i32,
    pub request: CurrentEventRequest,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub sacrifice: u8,
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
            sacrifice: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventType {
    ExploreResolution(ExploreResolutionState),
    InfluenceCultureResolution(ResourcePile),
    CombatStart(Combat),
    CombatRoundEnd(CombatRoundEnd),
    CombatEnd(CombatEnd),
    StatusPhase(StatusPhaseState),
    TurnStart,
    Advance(AdvanceInfo),
    Construct(Building),
    Recruit(Recruit),
    Incident(IncidentInfo),
    DrawWonderCard,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CurrentEventState {
    pub event_type: CurrentEventType,
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

impl CurrentEventState {
    #[must_use]
    pub fn new(current_player: usize, event_type: CurrentEventType) -> Self {
        Self {
            players_used: vec![],
            player: CurrentEventPlayer::new(current_player),
            barbarians: None,
            selected_player: None,
            event_type,
        }
    }
}

pub type PositionRequest = MultiRequest<Position>;

pub(crate) fn new_position_request(
    mut choices: Vec<Position>,
    needed: RangeInclusive<u8>,
    description: &str,
) -> PositionRequest {
    choices.sort();
    MultiRequest::new(choices, needed, description)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UnitTypeRequest {
    pub choices: Vec<UnitType>,
    pub player_index: usize,
    pub description: String,
}

impl UnitTypeRequest {
    #[must_use]
    pub fn new(choices: Vec<UnitType>, player_index: usize, description: &str) -> Self {
        Self {
            choices,
            player_index,
            description: description.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct UnitsRequest {
    pub player: usize,
    #[serde(flatten)]
    pub request: MultiRequest<u32>,
}

impl UnitsRequest {
    #[must_use]
    pub fn new(
        player: usize,
        choices: Vec<u32>,
        needed: RangeInclusive<u8>,
        description: &str,
    ) -> Self {
        Self {
            player,
            request: MultiRequest::new(choices, needed, description),
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
pub struct MultiRequest<T> {
    pub choices: Vec<T>,
    pub needed: RangeInclusive<u8>,
    pub description: String,
}

impl<T> MultiRequest<T> {
    #[must_use]
    pub fn new(choices: Vec<T>, needed: RangeInclusive<u8>, description: &str) -> Self {
        Self {
            choices,
            needed,
            description: description.to_string(),
        }
    }

    #[must_use]
    pub fn is_valid(&self, selected: &[T]) -> bool {
        self.needed.contains(&(selected.len() as u8))
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ChangeGovernmentRequest {
    pub optional: bool,
    pub cost: ResourcePile,
}

impl ChangeGovernmentRequest {
    #[must_use]
    pub fn new(optional: bool, cost: ResourcePile) -> Self {
        Self { optional, cost }
    }
}

impl CurrentEventResponse {
    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = game.current_event_handler_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let details = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, &details);
    }
}
