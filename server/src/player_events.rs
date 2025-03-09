use crate::advance::Advance;
use crate::collect::{CollectContext, CollectInfo};
use crate::combat::Combat;
use crate::combat_listeners::{CombatResultInfo, CombatRoundResult, CombatStrength};
use crate::events::Event;
use crate::explore::ExploreResolutionState;
use crate::game::Game;
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::{PlayingActionType, Recruit};
use crate::unit::Units;
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub(crate) type CurrentEvent<V = ()> = Event<Game, CurrentEventInfo, V>;

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: CurrentEvent<Building>,
    pub on_construct_wonder: Event<Player, Position, Wonder>,
    pub on_draw_wonder_card: CurrentEvent,
    pub on_collect: Event<CollectInfo, Game>,
    pub on_advance: CurrentEvent<AdvanceInfo>,
    pub on_recruit: CurrentEvent<Recruit>,
    pub on_influence_culture_attempt: Event<InfluenceCultureInfo, City, Game>,
    pub on_influence_culture_success: Event<Game, usize>,
    pub on_influence_culture_resolution: CurrentEvent<ResourcePile>,
    pub before_move: Event<Game, MoveInfo>,
    pub on_explore_resolution: CurrentEvent<ExploreResolutionState>,

    pub construct_cost: Event<CostInfo, City, Building>,
    pub wonder_cost: Event<CostInfo, City, Wonder>,
    pub advance_cost: Event<CostInfo, Advance>,
    pub happiness_cost: Event<CostInfo>,
    pub recruit_cost: Event<CostInfo, Units, Player>,

    pub is_playing_action_available: Event<bool, Game, PlayingActionInfo>,

    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<CollectInfo, CollectContext, Game>,
    pub collect_total: Event<CollectInfo>,

    pub on_status_phase: CurrentEvent,
    pub on_turn_start: CurrentEvent,
    pub on_incident: CurrentEvent<IncidentInfo>,
    pub on_combat_start: CurrentEvent,
    pub on_combat_round: Event<CombatStrength, Combat, Game>,
    pub on_combat_round_end: CurrentEvent<CombatRoundResult>,
    pub on_combat_end: CurrentEvent<CombatResultInfo>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}

#[derive(Clone, PartialEq)]
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
        let player = game.get_player_mut(self.player);
        for (k, v) in self.info.clone() {
            player.event_info.insert(k, v);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum IncidentTarget {
    ActivePlayer,
    AllPlayers,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct IncidentInfo {
    pub active_player: usize,
}

impl IncidentInfo {
    #[must_use]
    pub fn new(origin: usize) -> IncidentInfo {
        IncidentInfo {
            active_player: origin,
        }
    }

    #[must_use]
    pub fn is_active(&self, role: IncidentTarget, player: usize) -> bool {
        role == IncidentTarget::AllPlayers || self.active_player == player
    }
}

#[derive(Clone, PartialEq)]
pub struct CostInfo {
    pub cost: PaymentOptions,
    pub(crate) info: ActionInfo,
}

impl CostInfo {
    pub(crate) fn new(player: &Player, cost: PaymentOptions) -> CostInfo {
        CostInfo {
            cost,
            info: ActionInfo::new(player),
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
}

#[derive(Clone, PartialEq)]
pub struct CurrentEventInfo {
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
pub enum InfluenceCulturePossible {
    NoRestrictions,
    NoBoost,
    Impossible,
}

#[derive(Clone, PartialEq)]
pub struct InfluenceCultureInfo {
    pub is_defender: bool,
    pub possible: InfluenceCulturePossible,
    pub range_boost_cost: PaymentOptions,
    pub(crate) info: ActionInfo,
    pub roll_boost: u8,
}

impl InfluenceCultureInfo {
    #[must_use]
    pub(crate) fn new(range_boost_cost: PaymentOptions, info: ActionInfo) -> InfluenceCultureInfo {
        InfluenceCultureInfo {
            possible: InfluenceCulturePossible::NoRestrictions,
            range_boost_cost,
            info,
            roll_boost: 0,
            is_defender: false,
        }
    }

    #[must_use]
    pub fn is_possible(&self, range_boost: u32) -> bool {
        match self.possible {
            InfluenceCulturePossible::NoRestrictions => true,
            InfluenceCulturePossible::NoBoost => range_boost == 0,
            InfluenceCulturePossible::Impossible => false,
        }
    }

    pub fn set_impossible(&mut self) {
        self.possible = InfluenceCulturePossible::Impossible;
    }

    pub fn set_no_boost(&mut self) {
        if matches!(self.possible, InfluenceCulturePossible::Impossible) {
            return;
        }
        self.possible = InfluenceCulturePossible::NoBoost;
    }
}
