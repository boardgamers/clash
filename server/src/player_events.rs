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
    pub construct_cost: EventMut<ResourcePile, City, Building>,
    pub on_construct_wonder: EventMut<Player, Position, Wonder>,
    pub on_undo_construct_wonder: EventMut<Player, Position, Wonder>,
    pub wonder_cost: EventMut<PaymentOptions, City, Wonder>,
    pub on_advance: EventMut<Player, String, ()>,
    pub on_undo_advance: EventMut<Player, String, ()>,
    pub after_execute_action: EventMut<Player, Action, ()>,
    pub before_undo_action: EventMut<Player, Action, ()>,
    pub advance_cost: EventMut<u32, String>,
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
