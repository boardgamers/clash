use crate::action::execute_custom_phase_action;
use crate::action_card::ActionCardInfo;
use crate::card::HandCard;
use crate::city_pieces::Building;
use crate::combat::Combat;
use crate::combat_listeners::{CombatEnd, CombatRoundEnd, CombatRoundStart};
use crate::events::EventOrigin;
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::map::Rotation;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::{AdvanceInfo, IncidentInfo};
use crate::playing_actions::Recruit;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernmentType, StatusPhaseState};
use crate::unit::UnitType;
use crate::wonder::WonderCardInfo;
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

///
/// If a player does not own a hand card, then it means that it's a swap card from another player
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct HandCardsRequest {
    #[serde(flatten)]
    pub request: MultiRequest<HandCard>,
}

impl HandCardsRequest {
    #[must_use]
    pub fn new(cards: Vec<HandCard>, needed: RangeInclusive<u8>, description: &str) -> Self {
        HandCardsRequest {
            request: MultiRequest::new(cards, needed, description),
        }
    }
}

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
    SelectHandCards(HandCardsRequest),
    BoolRequest(String),
    ChangeGovernment(ChangeGovernmentRequest),
    ExploreResolution,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum EventResponse {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    SelectAdvance(String),
    SelectPlayer(usize),
    SelectPositions(Vec<Position>),
    SelectUnitType(UnitType),
    SelectUnits(Vec<u32>),
    SelectHandCards(Vec<HandCard>),
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
    pub response: Option<EventResponse>,
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
}

impl CurrentEventPlayer {
    #[must_use]
    pub fn new(current_player: usize) -> Self {
        Self {
            index: current_player,
            last_priority_used: None,
            handler: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CurrentEventType {
    ExploreResolution(ExploreResolutionState),
    InfluenceCultureResolution(ResourcePile),
    CombatStart(Combat),
    CombatRoundStart(CombatRoundStart),
    CombatRoundEnd(CombatRoundEnd),
    CombatEnd(CombatEnd),
    StatusPhase(StatusPhaseState),
    TurnStart,
    Advance(AdvanceInfo),
    Construct(Building),
    Recruit(Recruit),
    Incident(IncidentInfo),
    ActionCard(ActionCardInfo),
    WonderCard(WonderCardInfo),
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
}

impl CurrentEventState {
    #[must_use]
    pub fn new(current_player: usize, event_type: CurrentEventType) -> Self {
        Self {
            event_type,
            players_used: vec![],
            player: CurrentEventPlayer::new(current_player),
        }
    }

    #[must_use]
    pub fn active_player(&self) -> Option<&usize> {
        self.players_used.first()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PositionRequest {
    #[serde(flatten)]
    pub request: MultiRequest<Position>,
}

impl PositionRequest {
    #[must_use]
    pub fn new(mut choices: Vec<Position>, needed: RangeInclusive<u8>, description: &str) -> Self {
        choices.sort();
        PositionRequest {
            request: MultiRequest::new(choices, needed, description),
        }
    }
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

impl Structure {
    #[must_use]
    pub fn is_available(&self, player: &Player, game: &Game) -> bool {
        match self {
            Structure::CityCenter => player.is_city_available(),
            Structure::Building(b) => player.is_building_available(*b, game),
            Structure::Wonder(_) => false,
        }
    }
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

impl EventResponse {
    pub(crate) fn redo(self, game: &mut Game, player_index: usize) {
        let Some(s) = game.current_event_handler_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self.clone());
        let details = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, details);
    }
}
