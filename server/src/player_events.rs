use crate::action_card::ActionCardInfo;
use crate::advance::Advance;
use crate::barbarians::BarbariansEventState;
use crate::collect::{CollectContext, CollectInfo, PositionCollection};
use crate::combat::Combat;
use crate::combat_listeners::{CombatRoundEnd, CombatRoundStart};
use crate::combat_stats::CombatStats;
use crate::construct::ConstructInfo;
use crate::content::custom_actions::CustomActionActivation;
use crate::content::persistent_events::KilledUnits;
use crate::cultural_influence::{InfluenceCultureInfo, InfluenceCultureOutcome};
use crate::events::{Event, EventOrigin};
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::incident::PassedIncident;
use crate::map::Terrain;
use crate::objective_card::SelectObjectivesInfo;
use crate::payment::PaymentOptions;
use crate::playing_actions::{ActionPayment, PlayingActionType, Recruit};
use crate::status_phase::StatusPhaseState;
use crate::unit::Units;
use crate::utils;
use crate::wonder::{WonderBuildInfo, WonderCardInfo};
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile,
};
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub(crate) type PersistentEvent<V = ()> = Event<Game, PersistentEventInfo, (), V>;

pub(crate) struct PlayerEvents {
    pub persistent: PersistentEvents,
    pub transient: TransientEvents,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self {
            persistent: PersistentEvents::new(),
            transient: TransientEvents::new(),
        }
    }
}

pub(crate) struct TransientEvents {
    pub on_influence_culture_attempt: Event<Result<InfluenceCultureInfo, String>, City, Game>,
    pub on_influence_culture_resolve: Event<Game, InfluenceCultureOutcome>,
    pub before_move: Event<Game, MoveInfo>,

    pub building_cost: Event<CostInfo, Building, Game>,
    pub wonder_cost: Event<CostInfo, WonderBuildInfo, Game>,
    pub advance_cost: Event<CostInfo, Advance, Game>,
    pub happiness_cost: Event<CostInfo>,
    pub recruit_cost: Event<CostInfo, Units, Player>,

    pub is_playing_action_available: Event<Result<(), String>, Game, PlayingActionInfo>,

    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<CollectInfo, CollectContext, Game>,
    pub collect_total: Event<CollectInfo, Game, Vec<PositionCollection>>,
}

impl TransientEvents {
    pub fn new() -> TransientEvents {
        TransientEvents {
            on_influence_culture_attempt: Event::new("on_influence_culture_attempt"),
            on_influence_culture_resolve: Event::new("on_influence_culture_resolve"),
            before_move: Event::new("before_move"),

            building_cost: Event::new("building_cost"),
            wonder_cost: Event::new("wonder_cost"),
            advance_cost: Event::new("advance_cost"),
            happiness_cost: Event::new("happiness_cost"),
            recruit_cost: Event::new("recruit_cost"),

            is_playing_action_available: Event::new("is_playing_action_available"),

            terrain_collect_options: Event::new("terrain_collect_options"),
            collect_options: Event::new("collect_options"),
            collect_total: Event::new("collect_total"),
        }
    }
}

pub(crate) struct PersistentEvents {
    pub collect: PersistentEvent<CollectInfo>,
    pub construct: PersistentEvent<ConstructInfo>,
    pub draw_wonder_card: PersistentEvent<bool>,
    pub advance: PersistentEvent<OnAdvanceInfo>,
    pub recruit: PersistentEvent<Recruit>,
    pub found_city: PersistentEvent<Position>,
    pub influence_culture: PersistentEvent<InfluenceCultureInfo>,
    pub explore_resolution: PersistentEvent<ExploreResolutionState>,
    pub pay_action: PersistentEvent<ActionPayment>,
    pub play_action_card: PersistentEvent<ActionCardInfo>,
    pub play_wonder_card: PersistentEvent<WonderCardInfo>,

    pub status_phase: PersistentEvent<StatusPhaseState>,
    pub turn_start: PersistentEvent,
    pub incident: PersistentEvent<IncidentInfo>,
    pub stop_barbarian_movement: PersistentEvent<Vec<Position>>,
    pub combat_start: PersistentEvent<Combat>,
    pub combat_round_start_allow_tactics: PersistentEvent<CombatRoundStart>,
    pub combat_round_start: PersistentEvent<CombatRoundStart>,
    pub combat_round_start_reveal_tactics: PersistentEvent<CombatRoundStart>,
    pub combat_round_start_tactics: PersistentEvent<CombatRoundStart>,
    pub combat_round_end: PersistentEvent<CombatRoundEnd>,
    pub combat_round_end_tactics: PersistentEvent<CombatRoundEnd>,
    pub combat_end: PersistentEvent<CombatStats>,
    pub units_killed: PersistentEvent<KilledUnits>,
    pub select_objective_cards: PersistentEvent<SelectObjectivesInfo>,
    pub custom_action: PersistentEvent<CustomActionActivation>,
    pub choose_incident: PersistentEvent<IncidentInfo>,
    pub choose_action_card: PersistentEvent,
    pub city_activation_mood_decreased: PersistentEvent<Position>,
    pub ship_construction_conversion: PersistentEvent<Vec<u32>>,
}

impl PersistentEvents {
    #[must_use]
    pub fn new() -> PersistentEvents {
        PersistentEvents {
            collect: Event::new("collect"),
            construct: Event::new("construct"),
            draw_wonder_card: Event::new("draw_wonder_card"),
            advance: Event::new("advance"),
            recruit: Event::new("recruit"),
            found_city: Event::new("found_city"),
            influence_culture: Event::new("influence_culture"),
            explore_resolution: Event::new("explore_resolution"),
            pay_action: Event::new("pay_action"),
            play_action_card: Event::new("play_action_card"),
            play_wonder_card: Event::new("play_wonder_card"),

            status_phase: Event::new("status_phase"),
            turn_start: Event::new("turn_start"),
            incident: Event::new("incident"),
            stop_barbarian_movement: Event::new("stop_barbarian_movement"),
            combat_start: Event::new("combat_start"),
            combat_round_start: Event::new("combat_round_start"),
            combat_round_start_reveal_tactics: Event::new("combat_round_start_reveal_tactics"),
            combat_round_start_allow_tactics: Event::new("combat_round_start_allow_tactics"),
            combat_round_start_tactics: Event::new("combat_round_start_tactics"),
            combat_round_end: Event::new("combat_round_end"),
            combat_round_end_tactics: Event::new("combat_round_end_tactics"),
            combat_end: Event::new("combat_end"),
            units_killed: Event::new("units_killed"),
            select_objective_cards: Event::new("select_objective_cards"),

            custom_action: Event::new("custom_action_bartering"),
            choose_action_card: Event::new("great_mausoleum_action_card"),
            choose_incident: Event::new("great_mausoleum_incident"),
            city_activation_mood_decreased: Event::new("city_activation_mood_decreased"),
            ship_construction_conversion: Event::new("ship_construction_conversion"),
        }
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
    pub incident_id: u8,
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
    pub fn new(incident_id: u8, active_player: usize) -> IncidentInfo {
        IncidentInfo {
            incident_id,
            active_player,
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
    
    pub fn origin(&self) -> EventOrigin {
        EventOrigin::Incident(self.incident_id)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CostInfo {
    pub cost: PaymentOptions,
    pub activate_city: bool,
    pub(crate) ignore_required_advances: bool, // only used for wonder costs
    pub(crate) ignore_action_cost: bool,       // only used for wonder costs
    pub(crate) info: ActionInfo,
}

impl CostInfo {
    pub(crate) fn new(player: &Player, cost: PaymentOptions) -> CostInfo {
        CostInfo {
            cost,
            info: ActionInfo::new(player),
            activate_city: true,
            ignore_required_advances: false,
            ignore_action_cost: false,
        }
    }

    pub(crate) fn set_zero_resources(&mut self) {
        self.cost.default = ResourcePile::empty();
    }

    pub(crate) fn pay(&self, game: &mut Game, payment: &ResourcePile) {
        game.players[self.info.player].pay_cost(&self.cost, payment);
        self.info.execute(game);
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct OnAdvanceInfo {
    pub advance: Advance,
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

pub(crate) fn trigger_event_with_game_value<U, V, W>(
    game: &mut Game,
    player_index: usize,
    event: impl Fn(&mut PlayerEvents) -> &mut Event<Game, U, V, W>,
    info: &U,
    details: &V,
    extra_value: &mut W,
) where
    W: Clone + PartialEq,
{
    let e = event(&mut game.players[player_index].events).take();
    e.trigger(game, info, details, extra_value);
    event(&mut game.players[player_index].events).set(e);
}
