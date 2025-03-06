use crate::action::{execute_custom_phase_action, Action};
use crate::advance::undo_advance;
use crate::barbarians::BarbariansEventState;
use crate::city_pieces::Building;
use crate::combat_listeners::{CombatResult, CombatRoundResult};
use crate::content::advances::get_advance;
use crate::cultural_influence::undo_cultural_influence_resolution_action;
use crate::events::EventOrigin;
use crate::explore::{undo_explore_resolution, ExploreResolutionState};
use crate::game::Game;
use crate::map::Rotation;
use crate::payment::PaymentOptions;
use crate::player_events::{AdvanceInfo, IncidentInfo};
use crate::playing_actions::{PlayingAction, Recruit};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::ChangeGovernmentType;
use crate::undo::UndoContext;
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
    BoolRequest,
    ChangeGovernment(ChangeGovernmentRequest),
    ExploreResolution(ExploreResolutionState),
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
    CombatStart,
    CombatEnd(CombatResult),
    CombatRoundEnd(CombatRoundResult),
    StatusPhase,
    TurnStart,
    Advance(AdvanceInfo),
    Construct(Building),
    Recruit(Recruit),
    Incident(IncidentInfo),
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
    description: Option<String>,
) -> PositionRequest {
    choices.sort();
    MultiRequest::new(choices, needed, description)
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
    #[serde(flatten)]
    pub request: MultiRequest<u32>,
}

impl UnitsRequest {
    #[must_use]
    pub fn new(
        player: usize,
        choices: Vec<u32>,
        needed: RangeInclusive<u8>,
        description: Option<String>,
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl<T> MultiRequest<T> {
    #[must_use]
    pub fn new(choices: Vec<T>, needed: RangeInclusive<u8>, description: Option<String>) -> Self {
        Self {
            choices,
            needed,
            description,
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
}

impl ChangeGovernmentRequest {
    #[must_use]
    pub fn new(optional: bool) -> Self {
        Self { optional }
    }
}

impl CurrentEventResponse {
    pub(crate) fn undo(self, game: &mut Game, player_index: usize) {
        let Some(UndoContext::Event(e)) = game.pop_undo_context() else {
            panic!("when undoing custom phase event, the undo context stack should have a custom phase event")
        };
        let state = *e;
        match self {
            CurrentEventResponse::ExploreResolution(_r) => {
                if let Some(CurrentEventRequest::ExploreResolution(e)) =
                    state.player.handler.as_ref().map(|h| &h.request)
                {
                    undo_explore_resolution(game, player_index, e);
                } else {
                    panic!("explore resolution should have been requested")
                }
            }
            CurrentEventResponse::Payment(p) => {
                if let CurrentEventType::InfluenceCultureResolution(ref c) = state.event_type {
                    undo_cultural_influence_resolution_action(game, c);
                }

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
            | CurrentEventResponse::ChangeGovernmentType(_)
            | CurrentEventResponse::SelectUnits(_)
            | CurrentEventResponse::SelectPlayer(_)
            | CurrentEventResponse::SelectPositions(_)
            | CurrentEventResponse::SelectStructures(_)
            | CurrentEventResponse::SelectUnitType(_) => {
                // done with payer commands - or can't undo
            }
        }
        game.current_events.push(state);
        if game.action_log_index > 0 {
            if let Some(action) = game.action_log.get(game.action_log_index - 1) {
                // is there a better way to do this?
                if let Action::Playing(PlayingAction::Advance { .. }) = action.action {
                    game.players[player_index].incident_tokens += 1;
                }
            }
        }
    }

    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = game.current_event_handler_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let details = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, &details);
    }
}
