use crate::advance::Advance;
use crate::collect::{CollectContext, CollectInfo};
use crate::combat::{Combat, CombatResultInfo, CombatStrength};
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::events::Event;
use crate::game::{CommandUndoContext, CommandUndoInfo, Game};
use crate::map::Terrain;
use crate::payment::PaymentOptions;
use crate::playing_actions::{PlayingActionType, Recruit};
use crate::unit::{UnitType, Units};
use crate::{
    city::City, city_pieces::Building, player::Player, position::Position,
    resource_pile::ResourcePile, wonder::Wonder,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

pub(crate) type CustomPhaseEvent<V = ()> = Event<Game, CustomPhaseInfo, V>;

pub(crate) type PlayerCommandEvent<V = ()> = Event<PlayerCommands, Game, V>;

#[derive(Default)]
pub(crate) struct PlayerEvents {
    pub on_construct: CustomPhaseEvent<Building>,
    pub on_construct_wonder: Event<Player, Position, Wonder>,
    pub on_collect: PlayerCommandEvent<Position>,
    pub on_advance: PlayerCommandEvent<String>,
    pub on_advance_custom_phase: CustomPhaseEvent<AdvanceInfo>,
    pub on_recruit: CustomPhaseEvent<Recruit>,
    pub on_influence_culture_attempt: Event<InfluenceCultureInfo, City, Game>,
    pub on_influence_culture_success: PlayerCommandEvent,
    pub before_move: PlayerCommandEvent<MoveInfo>,

    pub construct_cost: Event<CostInfo, City, Building>,
    pub wonder_cost: Event<CostInfo, City, Wonder>,
    pub advance_cost: Event<CostInfo, Advance>,
    pub happiness_cost: Event<CostInfo>,
    pub recruit_cost: Event<CostInfo, Units, Player>,

    pub is_playing_action_available: Event<bool, PlayingActionType, Player>,

    pub terrain_collect_options: Event<HashMap<Terrain, HashSet<ResourcePile>>>,
    pub collect_options: Event<CollectInfo, CollectContext, Game>,
    pub collect_total: Event<CollectInfo>,

    pub on_turn_start: CustomPhaseEvent,
    pub on_combat_start: CustomPhaseEvent,
    pub on_combat_round: Event<CombatStrength, Combat, Game>,
    pub on_combat_end: CustomPhaseEvent<CombatResultInfo>,
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
        self.execute_with_options(game, |_| {});
    }

    pub(crate) fn execute_with_options(
        &self,
        game: &mut Game,
        c: impl Fn(&mut CommandUndoContext),
    ) {
        for l in self.log.iter().unique() {
            game.add_info_log_item(l);
        }
        let mut context = CommandUndoContext::new(self.info.clone());
        c(&mut context);
        self.undo.apply(game, context);
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

#[derive(Clone, PartialEq)]
pub(crate) struct PlayerCommands {
    pub name: String,
    pub index: usize,
    pub log: Vec<String>,
    pub content: CommandUndoContext,
}

impl PlayerCommands {
    #[must_use]
    pub fn new(player_index: usize, name: String, info: HashMap<String, String>) -> PlayerCommands {
        PlayerCommands {
            name,
            index: player_index,
            log: Vec::new(),
            content: CommandUndoContext::new(info),
        }
    }

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.content.gained_resources += resources;
    }

    pub fn gain_unit(&mut self, unit: UnitType, pos: Position) {
        self.content.gained_units.push((unit, pos));
    }

    pub fn add_info_log_item(&mut self, edit: &str) {
        self.log.push(edit.to_string());
    }
}
