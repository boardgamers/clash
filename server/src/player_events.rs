use crate::action::Action;
use crate::collect::CollectContext;
use crate::combat::{Combat, CombatStrength};
use crate::content::custom_phase_actions::{
    CurrentCustomPhaseEvent, CustomPhaseEventAction, CustomPhaseEventType,
};
use crate::game::Game;
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingActionType;
use crate::unit::Units;
use crate::{
    city::City, city_pieces::Building, events::EventMut, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use std::collections::{HashMap, HashSet};

type CustomPhaseEvent = EventMut<Game, usize, CustomPhaseEventType>;

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: EventMut<Player, Position, Building>,
    pub on_undo_construct: EventMut<Player, Position, Building>,
    pub on_construct_wonder: EventMut<Player, Position, Wonder>,
    pub on_undo_construct_wonder: EventMut<Player, Position, Wonder>,
    pub on_advance: EventMut<Player, String, ()>,
    pub on_undo_advance: EventMut<Player, String, ()>,
    pub before_move: EventMut<PlayerCommands, Game, MoveInfo>,
    pub after_execute_action: EventMut<Player, Action, ()>,
    pub before_undo_action: EventMut<Player, Action, ()>,

    pub construct_cost: EventMut<PaymentOptions, City, Building>,
    pub wonder_cost: EventMut<PaymentOptions, City, Wonder>,
    pub advance_cost: EventMut<u32, String>,
    pub happiness_cost: EventMut<PaymentOptions, (), ()>,
    pub recruit_cost: EventMut<RecruitCost, (), ()>,

    pub is_playing_action_available: EventMut<bool, PlayingActionType, Player>,
    pub terrain_collect_options: EventMut<HashMap<Terrain, HashSet<ResourcePile>>, (), ()>,
    pub collect_options: EventMut<HashMap<Position, HashSet<ResourcePile>>, CollectContext, Game>,
    pub on_turn_start: CustomPhaseEvent,
    pub on_combat_start: CustomPhaseEvent,
    pub on_combat_round: EventMut<CombatStrength, Combat, Game>,
    pub redo_custom_phase_action: EventMut<Game, CurrentCustomPhaseEvent, CustomPhaseEventAction>,
}

impl PlayerEvents {
    pub fn new() -> PlayerEvents {
        Self::default()
    }
}

#[derive(Clone, PartialEq)]
pub struct RecruitCost {
    pub cost: PaymentOptions,
    pub units: Units,
}

pub struct MoveInfo {
    pub player: usize,
    pub units: Vec<u32>,
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
    pub info: HashMap<String, String>,
    pub log_edits: Vec<String>,
    pub gained_resources: ResourcePile,
}

impl PlayerCommands {
    pub fn new(info: HashMap<String, String>) -> PlayerCommands {
        PlayerCommands {
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
