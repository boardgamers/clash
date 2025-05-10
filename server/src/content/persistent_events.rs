use crate::action::execute_custom_phase_action;
use crate::action_card::ActionCardInfo;
use crate::advance::Advance;
use crate::card::HandCard;
use crate::city_pieces::Building;
use crate::collect::CollectInfo;
use crate::combat::Combat;
use crate::combat_listeners::{CombatRoundEnd, CombatRoundStart};
use crate::combat_stats::CombatStats;
use crate::construct::ConstructInfo;
use crate::content::custom_actions::CustomEventAction;
use crate::cultural_influence::InfluenceCultureInfo;
use crate::events::EventOrigin;
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::map::Rotation;
use crate::objective_card::SelectObjectivesInfo;
use crate::payment::{PaymentOptions, ResourceReward};
use crate::player::Player;
use crate::player_events::{IncidentInfo, OnAdvanceInfo};
use crate::playing_actions::{PlayingAction, Recruit};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::status_phase::{ChangeGovernment, StatusPhaseState};
use crate::unit::UnitType;
use crate::wonder::{Wonder, WonderCardInfo};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum PersistentEventRequest {
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
    ChangeGovernment,
    ExploreResolution,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum EventResponse {
    Payment(Vec<ResourcePile>),
    ResourceReward(ResourcePile),
    SelectAdvance(Advance),
    SelectPlayer(usize),
    SelectPositions(Vec<Position>),
    SelectUnitType(UnitType),
    SelectUnits(Vec<u32>),
    SelectHandCards(Vec<HandCard>),
    SelectStructures(Vec<SelectedStructure>),
    Bool(bool),
    ChangeGovernmentType(ChangeGovernment),
    ExploreResolution(Rotation),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PersistentEventHandler {
    pub priority: i32,
    pub request: PersistentEventRequest,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<EventResponse>,
    pub origin: EventOrigin,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PersistentEventPlayer {
    #[serde(rename = "player")]
    pub index: usize,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_priority_used: Option<i32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<PersistentEventHandler>,
}

impl PersistentEventPlayer {
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
pub enum PersistentEventType {
    Collect(CollectInfo),
    ExploreResolution(ExploreResolutionState),
    InfluenceCulture(InfluenceCultureInfo),
    UnitsKilled(KilledUnits),
    CombatStart(Combat),
    CombatRoundStart(CombatRoundStart),
    CombatRoundEnd(CombatRoundEnd),
    EndCombat(CombatStats),
    StatusPhase(StatusPhaseState),
    TurnStart,
    PayAction(PlayingAction),
    Advance(OnAdvanceInfo),
    Construct(ConstructInfo),
    Recruit(Recruit),
    FoundCity(Position),
    Incident(IncidentInfo),
    ActionCard(ActionCardInfo),
    WonderCard(WonderCardInfo),
    DrawWonderCard,
    SelectObjectives(SelectObjectivesInfo),
    CustomAction(CustomEventAction),
    ChooseActionCard,
    ChooseIncident(IncidentInfo),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PersistentEventState {
    pub event_type: PersistentEventType,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub players_used: Vec<usize>,

    #[serde(flatten)]
    pub player: PersistentEventPlayer,
}

impl PersistentEventState {
    #[must_use]
    pub fn new(current_player: usize, event_type: PersistentEventType) -> Self {
        Self {
            event_type,
            players_used: vec![],
            player: PersistentEventPlayer::new(current_player),
        }
    }

    #[must_use]
    pub fn active_player(&self) -> Option<&usize> {
        self.players_used.first()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct KilledUnits {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub killer: Option<usize>,
    pub position: Position,
}

impl KilledUnits {
    #[must_use]
    pub fn new(position: Position, killer: Option<usize>) -> Self {
        Self { killer, position }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PaymentRequest {
    pub cost: PaymentOptions,
    pub name: String,
    pub optional: bool,
}

impl PaymentRequest {
    #[must_use]
    fn new(cost: PaymentOptions, name: &str, optional: bool) -> Self {
        Self {
            cost,
            name: name.to_string(),
            optional,
        }
    }

    #[must_use]
    pub fn mandatory(cost: PaymentOptions, name: &str) -> Self {
        Self::new(cost, name, false)
    }

    #[must_use]
    pub fn optional(cost: PaymentOptions, name: &str) -> Self {
        Self::new(cost, name, true)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ResourceRewardRequest {
    pub reward: ResourceReward,
    pub name: String,
}

impl ResourceRewardRequest {
    #[must_use]
    pub fn new(reward: ResourceReward, name: String) -> Self {
        Self { reward, name }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AdvanceRequest {
    pub choices: Vec<Advance>,
}

impl AdvanceRequest {
    #[must_use]
    pub fn new(mut choices: Vec<Advance>) -> Self {
        choices.sort();
        choices.dedup();
        Self { choices }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct SelectedStructure {
    pub position: Position,
    pub structure: Structure,
}

impl SelectedStructure {
    #[must_use]
    pub fn new(position: Position, structure: Structure) -> Self {
        Self {
            position,
            structure,
        }
    }
}

pub type StructuresRequest = MultiRequest<SelectedStructure>;

///
/// If a player does not own a hand card, then it means that it's a swap card from another player
pub type HandCardsRequest = MultiRequest<HandCard>;

#[must_use]
pub fn is_selected_structures_valid(game: &Game, selected: &[SelectedStructure]) -> bool {
    selected
        .iter()
        .chunk_by(|s| s.position)
        .into_iter()
        .all(|(p, g)| {
            let v = g.collect_vec();
            v.len() == game.get_any_city(p).size()
                || !v
                    .iter()
                    .any(|s| matches!(s.structure, Structure::CityCenter))
        })
}

pub type PositionRequest = MultiRequest<Position>;

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Structure {
    CityCenter,
    Building(Building),
    Wonder(Wonder),
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

impl<T: PartialEq + Ord> MultiRequest<T> {
    #[must_use]
    pub fn new(mut choices: Vec<T>, needed: RangeInclusive<u8>, description: &str) -> Self {
        choices.sort();
        choices.dedup();

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

impl EventResponse {
    pub(crate) fn redo(self, game: &mut Game, player_index: usize) -> Result<(), String> {
        let Some(s) = game.current_event_handler_mut() else {
            panic!("current custom phase event should be set")
        };
        s.response = Some(self);
        let details = game.current_event().event_type.clone();
        execute_custom_phase_action(game, player_index, details)
    }
}
