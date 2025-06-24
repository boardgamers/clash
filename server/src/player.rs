use crate::advance::{Advance, base_advance_cost, player_government};
use crate::city_pieces::DestroyedStructures;
use crate::consts::{STACK_LIMIT, UNIT_LIMIT_BARBARIANS, UNIT_LIMIT_PIRATES};
use crate::content::ability::construct_event_origin;
use crate::content::custom_actions::{CustomActionExecution, CustomActionInfo};
use crate::events::{Event, EventOrigin, EventPlayer};
use crate::leader::Leader;
use crate::leader_ability::LeaderAbility;
use crate::log::{ActionLogBalance, ActionLogEntry, add_action_log_item};
use crate::objective_card::CompletedObjective;
use crate::payment::PaymentOptions;
use crate::player_events::{CostInfo, TransientEvents};
use crate::playing_actions::PlayingActionType;
use crate::special_advance::SpecialAdvance;
use crate::unit::UnitType;
use crate::victory_points::{
    SpecialVictoryPoints, VictoryPointAttribution, add_special_victory_points, victory_points_parts,
};
use crate::wonder::Wonder;
use crate::{
    city::City,
    city_pieces::Building::{self},
    civilization::Civilization,
    consts::{BUILDING_COST, CITY_LIMIT, CITY_PIECE_LIMIT, UNIT_LIMIT},
    content::custom_actions::CustomActionType,
    game::Game,
    leader::LeaderInfo,
    player_events::PlayerEvents,
    position::Position,
    resource_pile::ResourcePile,
    unit::{Unit, Units},
};
use enumset::EnumSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayerType {
    Human,
    Barbarian,
}

pub struct Player {
    pub(crate) name: Option<String>,
    pub index: usize,
    pub resources: ResourcePile,
    pub resource_limit: ResourcePile,
    // transient, only for the current turn, only the active player can gain resources
    pub wasted_resources: ResourcePile,
    pub(crate) events: PlayerEvents,
    pub cities: Vec<City>,
    pub destroyed_structures: DestroyedStructures,
    pub units: Vec<Unit>,
    pub civilization: Civilization,
    pub available_leaders: Vec<Leader>,
    pub recruited_leaders: Vec<Leader>,
    pub advances: EnumSet<Advance>,
    pub great_library_advance: Option<Advance>,
    pub special_advances: EnumSet<SpecialAdvance>,
    pub wonders_built: Vec<Wonder>,
    pub wonders_owned: EnumSet<Wonder>, // transient
    pub incident_tokens: u8,
    pub completed_objectives: Vec<CompletedObjective>,
    pub captured_leaders: Vec<Leader>,
    pub special_victory_points: Vec<SpecialVictoryPoints>,
    pub custom_actions: HashMap<CustomActionType, CustomActionInfo>, // transient
    pub wonder_cards: Vec<Wonder>,
    pub action_cards: Vec<u8>,
    pub objective_cards: Vec<u8>,
    pub next_unit_id: u32,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub event_info: HashMap<String, String>,
    pub secrets: Vec<String>,
    pub custom_data: HashMap<String, Data>,
    pub(crate) objective_opportunities: Vec<String>, // transient
    pub(crate) gained_objective: Option<u8>,         // transient
    pub(crate) great_mausoleum_action_cards: u8,     // transient
}

impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum CostTrigger {
    WithModifiers,
    NoModifiers,
    // NoModifiersWithExtraListeners(Arc<dyn Fn(&CostInfo, &Wonder, &Game) + Send + Sync>),
}

impl Player {
    ///
    /// # Panics
    /// Panics if the civilization does not exist
    #[must_use]
    pub fn new(civilization: Civilization, index: usize) -> Self {
        Self {
            name: None,
            index,
            resources: ResourcePile::empty(),
            resource_limit: ResourcePile::empty(),
            wasted_resources: ResourcePile::empty(),
            events: PlayerEvents::new(),
            cities: Vec::new(),
            destroyed_structures: DestroyedStructures::new(),
            units: Vec::new(),
            available_leaders: civilization
                .leaders
                .iter()
                .map(|leader| leader.leader)
                .collect_vec(),
            recruited_leaders: Vec::new(),
            civilization,
            advances: EnumSet::empty(),
            special_advances: EnumSet::empty(),
            great_library_advance: None,
            incident_tokens: 0,
            completed_objectives: Vec::new(),
            captured_leaders: Vec::new(),
            special_victory_points: Vec::new(),
            custom_actions: HashMap::new(),
            wonder_cards: Vec::new(),
            action_cards: Vec::new(),
            objective_cards: Vec::new(),
            wonders_built: Vec::new(),
            wonders_owned: EnumSet::new(),
            next_unit_id: 0,
            played_once_per_turn_actions: Vec::new(),
            event_info: HashMap::new(),
            secrets: Vec::new(),
            custom_data: HashMap::new(),
            objective_opportunities: Vec::new(),
            gained_objective: None,
            great_mausoleum_action_cards: 0,
        }
    }

    ///
    /// # Panics
    ///
    /// Panics if the leader does not exist
    #[must_use]
    pub fn get_leader(&self, name: Leader) -> &LeaderInfo {
        self.civilization
            .leaders
            .iter()
            .find(|leader| leader.leader == name)
            .unwrap_or_else(|| panic!("Leader {name:?} not found"))
    }

    ///
    /// # Panics
    ///
    /// Panics if the leader ability does not exist
    #[must_use]
    pub fn get_leader_ability(&self, name: &str) -> &LeaderAbility {
        self.civilization
            .leaders
            .iter()
            .find_map(|leader| leader.abilities.iter().find(|l| l.name == name))
            .unwrap_or_else(|| panic!("Leader ability {name} not found"))
    }

    pub(crate) fn with_leader(
        leader: Leader,
        game: &mut Game,
        player_index: usize,
        f: impl FnOnce(&mut Game, &LeaderAbility) + Clone,
    ) {
        let pos = game.players[player_index]
            .civilization
            .leaders
            .iter()
            .position(|l| l.leader == leader)
            .expect("player should have the leader");
        let l = game.players[player_index].civilization.leaders.remove(pos);
        for a in &l.abilities {
            (f.clone())(game, a);
        }
        game.players[player_index]
            .civilization
            .leaders
            .insert(pos, l);
    }

    #[must_use]
    pub fn available_leaders(&self) -> Vec<&LeaderInfo> {
        self.available_leaders
            .iter()
            .map(|name| self.get_leader(*name))
            .collect()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    #[must_use]
    pub fn get_name(&self) -> String {
        if self.is_human() {
            self.name
                .clone()
                .unwrap_or(format!("Player{}", self.index + 1))
        } else {
            self.civilization.name.clone()
        }
    }

    /// Returns the government of this [`Player`].
    ///
    /// # Panics
    ///
    /// Panics if the player has advances which don't exist
    #[must_use]
    pub fn government(&self, game: &Game) -> Option<String> {
        player_government(game, self.advances)
    }

    #[must_use]
    pub fn can_afford(&self, cost: &PaymentOptions) -> bool {
        cost.can_afford(&self.resources)
    }

    #[must_use]
    pub fn can_advance_ignore_contradicting(&self, advance: Advance, game: &Game) -> bool {
        if self.has_advance(advance) {
            return false;
        }
        if let Some(required_advance) = advance.info(game).required {
            if !self.has_advance(required_advance) {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn can_advance_free(&self, advance: Advance, game: &Game) -> bool {
        for contradicting_advance in &advance.info(game).contradicting {
            if self.has_advance(*contradicting_advance) {
                return false;
            }
        }
        self.can_advance_ignore_contradicting(advance, game)
    }

    #[must_use]
    pub fn can_advance(&self, advance: Advance, game: &Game) -> bool {
        self.can_afford(
            &self
                .advance_cost(advance, game, CostTrigger::NoModifiers)
                .cost,
        ) && self.can_advance_free(advance, game)
    }

    #[must_use]
    pub fn has_advance(&self, advance: Advance) -> bool {
        self.advances.contains(advance)
    }

    #[must_use]
    pub fn has_special_advance(&self, advance: SpecialAdvance) -> bool {
        self.special_advances.contains(advance)
    }

    #[must_use]
    pub fn can_use_advance(&self, advance: Advance) -> bool {
        self.has_advance(advance) || self.great_library_advance.is_some_and(|a| a == advance)
    }

    #[must_use]
    pub fn victory_points(&self, game: &Game) -> f32 {
        victory_points_parts(self, game)
            .iter()
            .map(|(_, v)| v)
            .sum()
    }

    #[must_use]
    pub fn owned_buildings(&self, game: &Game) -> usize {
        game.players
            .iter()
            .flat_map(|player| &player.cities)
            .map(|city| city.pieces.buildings(Some(self.index)).len())
            .sum()
    }

    #[must_use]
    pub fn is_building_available(&self, building: Building, game: &Game) -> bool {
        game.players
            .iter()
            .flat_map(|player| &player.cities)
            .flat_map(|city| city.pieces.building_owners())
            .filter(|(b, owner)| b == &building && owner.is_some_and(|owner| owner == self.index))
            .count()
            < CITY_PIECE_LIMIT - self.destroyed_structures.get_building(building)
    }

    #[must_use]
    pub fn is_city_available(&self) -> bool {
        self.cities.len() < (CITY_LIMIT - self.destroyed_structures.cities) as usize
    }

    #[must_use]
    pub fn is_human(&self) -> bool {
        self.civilization.is_human()
    }

    #[must_use]
    pub fn available_units(&self) -> Units {
        let mut units = self.unit_limit();
        for u in &self.units {
            units -= &u.unit_type;
        }
        units
    }

    #[must_use]
    pub fn unit_limit(&self) -> Units {
        if self.is_human() {
            UNIT_LIMIT.clone()
        } else if self.civilization.is_barbarian() {
            UNIT_LIMIT_BARBARIANS.clone()
        } else {
            UNIT_LIMIT_PIRATES.clone()
        }
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = self.wonder_cards.iter().map(|_| Wonder::Hidden).collect();
        self.action_cards = self.action_cards.iter().map(|_| 0).collect();
        self.objective_cards = self.objective_cards.iter().map(|_| 0).collect();
        self.secrets = Vec::new();
    }

    #[must_use]
    pub fn building_cost(&self, game: &Game, building: Building, execute: CostTrigger) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.building_cost,
            CostInfo::new(
                self,
                PaymentOptions::resources(self, construct_event_origin(), BUILDING_COST),
            ),
            &building,
            game,
            execute,
        )
    }

    #[must_use]
    pub fn advance_cost(&self, advance: Advance, game: &Game, execute: CostTrigger) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.advance_cost,
            CostInfo::new(self, base_advance_cost(self)),
            &advance,
            game,
            execute,
        )
    }

    #[must_use]
    pub fn try_get_city(&self, position: Position) -> Option<&City> {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)?;
        Some(&self.cities[position])
    }

    ///
    /// # Panics
    /// Panics if city does not exist
    #[must_use]
    pub fn get_city(&self, position: Position) -> &City {
        self.try_get_city(position).expect("city should exist")
    }

    #[must_use]
    pub fn try_get_city_mut(&mut self, position: Position) -> Option<&mut City> {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)?;
        Some(&mut self.cities[position])
    }

    ///
    /// # Panics
    /// Panics if city does not exist
    #[must_use]
    pub fn get_city_mut(&mut self, position: Position) -> &mut City {
        self.try_get_city_mut(position).expect("city should exist")
    }

    #[must_use]
    pub fn can_raze_city(&self, city_position: Position) -> bool {
        self.cities.len() > 1
            && self
                .try_get_city(city_position)
                .is_some_and(|city| city.size() == 1)
    }

    #[must_use]
    pub fn try_get_unit(&self, id: u32) -> Option<&Unit> {
        self.units.iter().find(|unit| unit.id == id)
    }

    /// Returns an immutable reference to a player's unit.
    ///
    /// # Panics
    /// Panics if unit does not exist
    #[must_use]
    pub fn get_unit(&self, id: u32) -> &Unit {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .unwrap_or_else(|| panic!("unit should exist {id} for player {}", self.index))
    }

    /// Returns a mutable reference to a player's unit.
    ///
    /// # Panics
    /// Panics if unit does not exist
    #[must_use]
    pub fn get_unit_mut(&mut self, id: u32) -> &mut Unit {
        self.units
            .iter_mut()
            .find(|unit| unit.id == id)
            .unwrap_or_else(|| panic!("unit should exist {id}for player {}", self.index))
    }

    #[must_use]
    pub fn get_units(&self, position: Position) -> Vec<&Unit> {
        self.units
            .iter()
            .filter(|unit| unit.position == position)
            .collect()
    }

    #[must_use]
    pub fn get_units_mut(&mut self, position: Position) -> Vec<&mut Unit> {
        self.units
            .iter_mut()
            .filter(|unit| unit.position == position)
            .collect()
    }

    #[must_use]
    pub fn active_leader(&self) -> Option<Leader> {
        self.units.iter().find_map(|unit| {
            if let UnitType::Leader(l) = unit.unit_type {
                Some(l)
            } else {
                None
            }
        })
    }

    pub(crate) fn trigger_event<T, U, V>(
        &self,
        event: fn(&TransientEvents) -> &Event<T, U, V, ()>,
        value: &mut T,
        info: &U,
        details: &V,
    ) where
        T: Clone + PartialEq,
    {
        let e = event(&self.events.transient);
        e.get().trigger(value, info, details, &mut ());
    }

    pub(crate) fn trigger_cost_event<U, V>(
        &self,
        get_event: impl Fn(&TransientEvents) -> &Event<CostInfo, U, V>,
        mut cost_info: CostInfo,
        info: &U,
        details: &V,
        trigger: CostTrigger,
    ) -> CostInfo {
        let event = get_event(&self.events.transient).get();
        match trigger {
            CostTrigger::WithModifiers => {
                let m =
                    event.trigger_with_modifiers(&mut cost_info, info, details, &mut (), trigger);
                cost_info.cost.modifiers = m;
            }
            CostTrigger::NoModifiers => {
                event.trigger(&mut cost_info, info, details, &mut ());
            }
        }
        cost_info
    }

    ///
    /// # Panics
    /// Panics if the custom action type does not exist for this player
    #[must_use]
    pub fn custom_action_info(&self, custom_action_type: CustomActionType) -> CustomActionInfo {
        self.custom_actions
            .get(&custom_action_type)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "Custom action {custom_action_type:?} not found for player {}",
                    self.index
                )
            })
    }

    #[must_use]
    pub(crate) fn custom_action_modifiers(
        &self,
        base: &PlayingActionType,
    ) -> Vec<CustomActionType> {
        self.custom_actions
            .iter()
            .filter_map(move |(t, c)| {
                if let CustomActionExecution::Modifier(b) = &c.execution {
                    (b == base).then_some(*t)
                } else {
                    None
                }
            })
            .collect_vec()
    }

    pub(crate) fn gain_event_victory_points(&mut self, points: f32, origin: &EventOrigin) {
        add_special_victory_points(self, points, origin, VictoryPointAttribution::Events);
    }

    pub(crate) fn gain_objective_victory_points(&mut self, points: f32, origin: &EventOrigin) {
        add_special_victory_points(self, points, origin, VictoryPointAttribution::Objectives);
    }
}

pub(crate) fn gain_unit(
    game: &mut Game,
    player: &EventPlayer,
    position: Position,
    unit_type: UnitType,
) {
    gain_units(
        game,
        player.index,
        position,
        Units::from_iter(vec![unit_type]),
        &player.origin,
    );
}

pub fn gain_units(
    game: &mut Game,
    player: usize,
    position: Position,
    units: Units,
    origin: &EventOrigin,
) {
    for (unit_type, amount) in units.clone() {
        for _ in 0..amount {
            if let UnitType::Leader(leader) = &unit_type {
                let p = game.player_mut(player);
                p.available_leaders.retain(|name| name != leader);
                p.recruited_leaders.push(*leader);
                Player::with_leader(*leader, game, player, |game, leader| {
                    leader.listeners.init_first(game, player);
                });
            }
            let p = game.player_mut(player);
            p.units
                .push(Unit::new(player, position, unit_type, p.next_unit_id));
            p.next_unit_id += 1;
        }
    }

    game.log(
        player,
        origin,
        &format!("Gain {} at {}", units.to_string(Some(game)), position),
    );
    add_action_log_item(
        game,
        player,
        ActionLogEntry::units(units, ActionLogBalance::Gain),
        origin.clone(),
        vec![],
    );
}

pub(crate) fn remove_unit(player: usize, id: u32, game: &mut Game) -> Unit {
    // carried units can be transferred to another ship - which has to be selected later
    let p = game.player_mut(player);

    p.units.remove(
        p.units
            .iter()
            .position(|unit| unit.id == id)
            .expect("unit should exist"),
    )
}

pub fn end_turn(game: &mut Game, player: usize) {
    let p = game.player_mut(player);
    for city in &mut p.cities {
        city.deactivate();
    }
    for unit in &mut p.units {
        unit.movement_restrictions = vec![];
    }
    p.played_once_per_turn_actions.clear();
    p.event_info.clear();
    if let Some(a) = p.great_library_advance.take() {
        a.info(game).listeners.clone().deinit(game, player);
    }
}

pub(crate) fn can_add_army_unit(p: &Player, position: Position) -> bool {
    p.get_units(position)
        .iter()
        .filter(|u| u.is_army_unit())
        .count()
        < STACK_LIMIT
}

pub enum Data {
    Number(u32),
    Positions(Vec<Position>),
}

impl Data {
    pub fn number(&self) -> u32 {
        if let Self::Number(v) = self {
            *v
        } else {
            panic!("Data is not a number")
        }
    }
    pub fn number_mut(&mut self) -> &mut u32 {
        if let Self::Number(v) = self {
            v
        } else {
            panic!("Data is not a number")
        }
    }

    pub fn positions(&self) -> &Vec<Position> {
        if let Self::Positions(v) = self {
            v
        } else {
            panic!("Data is not positions")
        }
    }
    pub fn positions_mut(&mut self) -> &mut Vec<Position> {
        if let Self::Positions(v) = self {
            v
        } else {
            panic!("Data is not positions")
        }
    }
}
