use crate::action_card::ActionCardInfo;
use crate::advance::Advance;
use crate::barbarians::BarbariansEventState;
use crate::card::HandCard;
use crate::collect::{CollectContext, CollectInfo};
use crate::combat::Combat;
use crate::combat_listeners::{CombatEnd, CombatRoundEnd, CombatRoundStart};
use crate::content::persistent_events::{KilledUnits, Structure};
use crate::events::Event;
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::incident::PassedIncident;
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::{PlayingActionType, Recruit};
use crate::status_phase::StatusPhaseState;
use crate::unit::Units;
use crate::utils;
use crate::wonder::WonderCardInfo;
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile,
};
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub(crate) type PersistentEvent<V = ()> = Event<Game, PersistentEventInfo, (), V>;

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub persistent: PersistentEvents,
    pub transient: TransientEvents,
}

#[derive(Default)]
pub(crate) struct TransientEvents {
    pub on_influence_culture_attempt: Event<InfluenceCultureInfo, City, Game>,
    pub on_influence_culture_resolve: Event<Game, InfluenceCultureOutcome>,
    pub before_move: Event<Game, MoveInfo>,

    pub construct_cost: Event<CostInfo, Building, Game>,
    pub advance_cost: Event<CostInfo, Advance>,
    pub happiness_cost: Event<CostInfo>,
    pub recruit_cost: Event<CostInfo, Units, Player>,

    pub is_playing_action_available: Event<Result<(), String>, Game, PlayingActionInfo>,

    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<CollectInfo, CollectContext, Game>,
    pub collect_total: Event<CollectInfo>,
}

#[derive(Default)]
pub(crate) struct PersistentEvents {
    pub collect: PersistentEvent<CollectInfo>,
    pub construct: PersistentEvent<Building>,
    pub draw_wonder_card: PersistentEvent,
    pub advance: PersistentEvent<AdvanceInfo>,
    pub recruit: PersistentEvent<Recruit>,
    pub influence_culture_resolution: PersistentEvent<ResourcePile>,
    pub explore_resolution: PersistentEvent<ExploreResolutionState>,
    pub play_action_card: PersistentEvent<ActionCardInfo>,
    pub play_wonder_card: PersistentEvent<WonderCardInfo>,

    pub status_phase: PersistentEvent<StatusPhaseState>,
    pub turn_start: PersistentEvent,
    pub incident: PersistentEvent<IncidentInfo>,
    pub combat_start: PersistentEvent<Combat>,
    pub combat_round_start: PersistentEvent<CombatRoundStart>,
    pub combat_round_start_reveal_tactics: PersistentEvent<CombatRoundStart>,
    pub combat_round_start_tactics: PersistentEvent<CombatRoundStart>,
    pub combat_round_end: PersistentEvent<CombatRoundEnd>,
    pub combat_round_end_tactics: PersistentEvent<CombatRoundEnd>,
    pub combat_end: PersistentEvent<CombatEnd>,
    pub units_killed: PersistentEvent<KilledUnits>,
    pub select_hand_cards: PersistentEvent<Vec<HandCard>>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub(crate) struct ActionInfo {
    pub(crate) player: usize,
    pub(crate) info: HashMap<String, String>,
    pub(crate) log: Vec<String>,
}

impl ActionInfo {
    pub(crate) fn new(player: &Player) -> ActionInfo {
        ActionInfo {
            player: player.index,
            info: player.event_info.clone(),
            log: Vec::new(),
        }
    }

    pub(crate) fn execute(&self, game: &mut Game) {
        for l in self.log.iter().unique() {
            game.add_info_log_item(l);
        }
        let player = game.player_mut(self.player);
        for (k, v) in self.info.clone() {
            player.event_info.insert(k, v);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum IncidentTarget {
    ActivePlayer,
    SelectedPlayer,
    AllPlayers,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct IncidentPlayerInfo {
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub sacrifice: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub myths_payment: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub must_reduce_mood: Vec<Position>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub payment: ResourcePile,
}

impl Default for IncidentPlayerInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl IncidentPlayerInfo {
    #[must_use]
    pub fn new() -> IncidentPlayerInfo {
        IncidentPlayerInfo {
            sacrifice: 0,
            myths_payment: 0,
            must_reduce_mood: Vec::new(),
            payment: ResourcePile::empty(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct IncidentInfo {
    pub active_player: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passed: Option<PassedIncident>,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub consumed: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barbarians: Option<BarbariansEventState>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_player: Option<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<Position>,

    #[serde(flatten)]
    pub player: IncidentPlayerInfo,
}

impl IncidentInfo {
    #[must_use]
    pub fn new(origin: usize) -> IncidentInfo {
        IncidentInfo {
            active_player: origin,
            passed: None,
            consumed: false,
            barbarians: None,
            selected_player: None,
            selected_position: None,
            player: IncidentPlayerInfo::new(),
        }
    }

    #[must_use]
    pub fn is_active(&self, role: IncidentTarget, player: usize) -> bool {
        if self.consumed || matches!(self.passed, Some(PassedIncident::NewPlayer(_))) {
            // wait until the new player is playing the advance
            return false;
        }

        match role {
            IncidentTarget::ActivePlayer => self.active_player == player,
            IncidentTarget::SelectedPlayer => self.selected_player == Some(player),
            IncidentTarget::AllPlayers => true,
        }
    }

    pub(crate) fn get_barbarian_state(&mut self) -> &mut BarbariansEventState {
        self.barbarians.as_mut().expect("barbarians should exist")
    }
}

#[derive(Clone, PartialEq)]
pub struct CostInfo {
    pub cost: PaymentOptions,
    pub activate_city: bool,
    pub(crate) info: ActionInfo,
}

impl CostInfo {
    pub(crate) fn new(player: &Player, cost: PaymentOptions) -> CostInfo {
        CostInfo {
            cost,
            info: ActionInfo::new(player),
            activate_city: true,
        }
    }

    pub(crate) fn set_zero(&mut self) {
        self.cost.default = ResourcePile::empty();
    }

    pub(crate) fn pay(&self, game: &mut Game, payment: &ResourcePile) {
        game.players[self.info.player].pay_cost(&self.cost, payment);
        self.info.execute(game);
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AdvanceInfo {
    pub name: String,
    pub payment: ResourcePile,
    pub take_incident_token: bool,
}

#[derive(Clone, PartialEq)]
pub struct PersistentEventInfo {
    pub player: usize, // player currently handling the event
}

pub struct MoveInfo {
    pub player: usize,
    pub units: Vec<u32>,
    #[allow(dead_code)]
    pub from: Position,
    pub to: Position,
}

impl MoveInfo {
    #[must_use]
    pub fn new(player: usize, units: Vec<u32>, from: Position, to: Position) -> MoveInfo {
        MoveInfo {
            player,
            units,
            from,
            to,
        }
    }
}

pub struct PlayingActionInfo {
    pub player: usize,
    pub action_type: PlayingActionType,
}

#[derive(Clone, PartialEq)]
pub struct InfluenceCultureInfo {
    pub is_defender: bool,
    pub structure: Structure,
    pub blockers: Vec<String>,
    pub prevent_boost: bool,
    pub range_boost_cost: PaymentOptions,
    pub(crate) info: ActionInfo,
    pub roll_boost: u8,
    pub position: Position,
}

impl InfluenceCultureInfo {
    #[must_use]
    pub(crate) fn new(
        range_boost_cost: PaymentOptions,
        info: ActionInfo,
        position: Position,
        structure: Structure,
    ) -> InfluenceCultureInfo {
        InfluenceCultureInfo {
            prevent_boost: false,
            structure,
            range_boost_cost,
            info,
            roll_boost: 0,
            is_defender: false,
            blockers: Vec::new(),
            position,
        }
    }

    pub fn add_blocker(&mut self, reason: &str) {
        self.blockers.push(reason.to_string());
    }

    pub fn set_no_boost(&mut self) {
        self.prevent_boost = true;
    }
}

#[derive(Clone, PartialEq)]
pub struct InfluenceCultureOutcome {
    pub success: bool,
    pub player: usize,
    pub position: Position,
}

impl InfluenceCultureOutcome {
    #[must_use]
    pub fn new(success: bool, player: usize, position: Position) -> InfluenceCultureOutcome {
        InfluenceCultureOutcome {
            success,
            player,
            position,
        }
    }
}
