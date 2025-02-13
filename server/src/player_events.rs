use crate::advance::Advance;
use crate::collect::{CollectContext, CollectOptionsInfo};
use crate::combat::{Combat, CombatStrength};
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::events::Event;
use crate::game::{CommandUndoContext, CommandUndoInfo, Game};
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::PlayingActionType;
use crate::unit::Units;
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: Event<Game, CustomPhaseInfo, Building>,
    pub on_construct_wonder: Event<Player, Position, Wonder>,
    pub on_collect: Event<PlayerCommands, Game, Position>,
    pub on_advance: Event<PlayerCommands, Game, String>,
    pub on_advance_custom_phase: Event<Game, CustomPhaseInfo, AdvanceInfo>,
    pub before_move: Event<PlayerCommands, Game, MoveInfo>,

    pub construct_cost: Event<CostInfo, City, Building>,
    pub wonder_cost: Event<CostInfo, City, Wonder>,
    pub advance_cost: Event<CostInfo, Advance>,
    pub happiness_cost: Event<CostInfo>,
    pub recruit_cost: Event<CostInfo, Units, Player>,

    pub is_playing_action_available: Event<bool, PlayingActionType, Player>,
    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<CollectOptionsInfo, CollectContext, Game>,
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
pub(crate) struct ActionInfo {
    pub(crate) player: usize,
    pub(crate) undo: CommandUndoInfo,
    pub(crate) info: HashMap<String, String>,
    pub(crate) log: Vec<String>,
}

impl ActionInfo {
    pub(crate) fn new(player: &Player) -> ActionInfo {
        ActionInfo {
            player: player.index,
            undo: CommandUndoInfo::new(player),
            info: player.event_info.clone(),
            log: Vec::new(),
        }
    }

    pub(crate) fn execute(&self, game: &mut Game) {
        for l in self.log.iter().unique() {
            game.add_info_log_item(l);
        }
        self.undo.apply(
            game,
            CommandUndoContext {
                info: self.info.clone(),
                gained_resources: ResourcePile::empty(),
            },
        );
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
        game.players[self.info.undo.player].pay_cost(&self.cost, payment);
        self.info.execute(game);
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
    pub log: Vec<String>,
    pub gained_resources: ResourcePile,
}

impl PlayerCommands {
    pub fn new(player_index: usize, name: String, info: HashMap<String, String>) -> PlayerCommands {
        PlayerCommands {
            name,
            index: player_index,
            info,
            log: Vec::new(),
            gained_resources: ResourcePile::default(),
        }
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.gained_resources += resources;
    }

    pub fn add_info_log_item(&mut self, edit: &str) {
        self.log.push(edit.to_string());
    }
}
