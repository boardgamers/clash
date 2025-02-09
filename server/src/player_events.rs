use crate::action::Action;
use crate::collect::CollectContext;
use crate::combat::{Combat, CombatStrength};
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::events::Event;
use crate::game::Game;
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingActionType;
use crate::unit::Units;
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: Event<Game, CustomPhaseInfo, Building>,
    pub on_construct_wonder: Event<Player, Position, Wonder>,
    pub on_advance: Event<PlayerCommands, Game, String>,
    pub on_advance_custom_phase: Event<Game, CustomPhaseInfo, AdvanceInfo>,
    pub before_move: Event<PlayerCommands, Game, MoveInfo>,
    pub after_execute_action: Event<Player, Action>,
    pub before_undo_action: Event<Player, Action>,

    pub construct_cost: Event<PaymentOptions, City, Building>,
    pub wonder_cost: Event<PaymentOptions, City, Wonder>,
    pub advance_cost: Event<AdvanceCostInfo>,
    pub happiness_cost: Event<PaymentOptions>,
    pub recruit_cost: Event<RecruitCost>,

    pub is_playing_action_available: Event<bool, PlayingActionType, Player>,
    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<HashMap<Position, HashSet<ResourcePile>>, CollectContext, Game>,
    pub on_turn_start: Event<Game, CustomPhaseInfo>,
    pub on_combat_start: Event<Game, CustomPhaseInfo>,
    pub on_combat_round: Event<CombatStrength, Combat, Game>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}

#[derive(Clone, PartialEq)]
pub struct AdvanceCostInfo {
    pub name: String,
    pub cost: PaymentOptions,
    pub info: HashMap<String, String>,
}

impl AdvanceCostInfo {
    pub fn set_cost(&mut self, cost: u32) {
        self.cost.default.ideas = cost;
    }
}

#[derive(Clone, PartialEq)]
pub struct AdvanceInfo {
    pub name: String,
    pub payment: ResourcePile,
}

#[derive(Clone, PartialEq)]
pub struct CustomPhaseInfo {
    pub event_type: CustomPhaseEventType,
    pub player: usize,
}

#[derive(Clone, PartialEq)]
pub struct RecruitCost {
    pub cost: PaymentOptions,
    pub units: Units,
}

pub struct MoveInfo {
    pub player: usize,
    pub units: Vec<u32>,
    #[allow(dead_code)]
    pub from: Position,
    pub to: Position,
}

impl MoveInfo {
    pub fn new(player: usize, units: Vec<u32>, from: Position, to: Position) -> MoveInfo {
        MoveInfo {
            player,
            units,
            from,
            to,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct PlayerCommands {
    pub name: String,
    pub index: usize,
    pub info: HashMap<String, String>,
    pub log_edits: Vec<String>,
    pub gained_resources: ResourcePile,
}

impl PlayerCommands {
    pub fn new(player_index: usize, name: String, info: HashMap<String, String>) -> PlayerCommands {
        PlayerCommands {
            name,
            index: player_index,
            info,
            log_edits: Vec::new(),
            gained_resources: ResourcePile::default(),
        }
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.gained_resources += resources;
    }

    pub fn add_to_last_log_item(&mut self, edit: &str) {
        self.log_edits.push(edit.to_string());
    }
}
