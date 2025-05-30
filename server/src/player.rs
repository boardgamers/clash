use crate::advance::{Advance, base_advance_cost, player_government};
use crate::city_pieces::DestroyedStructures;
use crate::consts::{STACK_LIMIT, UNIT_LIMIT_BARBARIANS, UNIT_LIMIT_PIRATES};
use crate::events::{Event, EventOrigin};
use crate::leader::{Leader, LeaderAbility};
use crate::payment::{PaymentOptions, PaymentReason};
use crate::player_events::{CostInfo, TransientEvents};
use crate::special_advance::SpecialAdvance;
use crate::unit::UnitType;
use crate::wonder::{Wonder, wonders_built_points, wonders_owned_points};
use crate::{
    city::City,
    city_pieces::Building::{self},
    civilization::Civilization,
    consts::{
        ADVANCE_VICTORY_POINTS, BUILDING_COST, BUILDING_VICTORY_POINTS,
        CAPTURED_LEADER_VICTORY_POINTS, CITY_LIMIT, CITY_PIECE_LIMIT, OBJECTIVE_VICTORY_POINTS,
        UNIT_LIMIT,
    },
    content::custom_actions::CustomActionType,
    game::Game,
    leader::LeaderInfo,
    player_events::PlayerEvents,
    position::Position,
    resource_pile::ResourcePile,
    unit::{Unit, Units},
    utils,
};
use enumset::EnumSet;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering::{self, *};
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
    pub active_leader: Option<String>, // todo remove this, it is not used
    pub available_leaders: Vec<Leader>,
    pub advances: EnumSet<Advance>,
    pub great_library_advance: Option<Advance>,
    pub special_advances: EnumSet<SpecialAdvance>,
    pub wonders_built: Vec<Wonder>,
    pub wonders_owned: EnumSet<Wonder>, // transient
    pub incident_tokens: u8,
    pub completed_objectives: Vec<String>,
    pub captured_leaders: Vec<String>,
    pub event_victory_points: f32,
    pub custom_actions: HashMap<CustomActionType, EventOrigin>,
    pub wonder_cards: Vec<Wonder>,
    pub action_cards: Vec<u8>,
    pub objective_cards: Vec<u8>,
    pub next_unit_id: u32,
    pub played_once_per_turn_actions: Vec<CustomActionType>,
    pub event_info: HashMap<String, String>,
    pub secrets: Vec<String>,
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
            active_leader: None,
            available_leaders: civilization
                .leaders
                .iter()
                .map(|l| l.name.clone())
                .collect(),
            civilization,
            advances: EnumSet::empty(),
            special_advances: EnumSet::empty(),
            great_library_advance: None,
            incident_tokens: 0,
            completed_objectives: Vec::new(),
            captured_leaders: Vec::new(),
            event_victory_points: 0.0,
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
    pub fn get_leader(&self, name: &str) -> &LeaderInfo {
        self.civilization
            .leaders
            .iter()
            .find(|leader| leader.name == name)
            .unwrap_or_else(|| panic!("Leader {name} not found"))
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
            .position(|l| l.name == leader)
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
            .map(|name| self.get_leader(name))
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

    pub fn gain_resources(&mut self, resources: ResourcePile) {
        self.resources += resources;
        let waste = self.resources.apply_resource_limit(&self.resource_limit);
        self.wasted_resources += waste;
    }

    #[must_use]
    pub fn can_afford(&self, cost: &PaymentOptions) -> bool {
        cost.can_afford(&self.resources)
    }

    pub(crate) fn pay_cost(&mut self, cost: &PaymentOptions, payment: &ResourcePile) {
        assert!(cost.can_afford(payment), "invalid payment - got {payment}");
        assert!(
            cost.is_valid_payment(payment),
            "Invalid payment - got {payment} for default cost {}",
            cost.default
        );
        self.lose_resources(payment.clone());
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if player cannot afford the resources
    pub(crate) fn lose_resources(&mut self, resources: ResourcePile) {
        assert!(
            self.resources.has_at_least(&resources),
            "player should be able to pay {resources} - got {}",
            self.resources
        );
        self.resources -= resources;
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
        self.victory_points_parts(game).iter().map(|(_, v)| v).sum()
    }

    #[must_use]
    pub fn victory_points_parts(&self, game: &Game) -> [(&'static str, f32); 6] {
        [
            (
                "City pieces",
                (self.cities.len() + self.owned_buildings(game)) as f32 * BUILDING_VICTORY_POINTS,
            ),
            (
                "Advances",
                (self.advances.len() + self.special_advances.len()) as f32 * ADVANCE_VICTORY_POINTS,
            ),
            (
                "Objectives",
                self.completed_objectives.len() as f32 * OBJECTIVE_VICTORY_POINTS,
            ),
            (
                "Wonders",
                wonders_owned_points(self, game) as f32 + wonders_built_points(self, game),
            ),
            ("Events", self.event_victory_points),
            (
                "Captured Leaders",
                self.captured_leaders.len() as f32 * CAPTURED_LEADER_VICTORY_POINTS,
            ),
        ]
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

    pub fn remove_wonder(&mut self, wonder: Wonder) {
        utils::remove_element(&mut self.wonders_built, &wonder);
        self.wonders_owned.remove(wonder);
    }

    pub fn strip_secret(&mut self) {
        self.wonder_cards = self.wonder_cards.iter().map(|_| Wonder::Hidden).collect();
        self.action_cards = self.action_cards.iter().map(|_| 0).collect();
        self.objective_cards = self.objective_cards.iter().map(|_| 0).collect();
        self.secrets = Vec::new();
    }

    #[must_use]
    pub(crate) fn compare_score(&self, other: &Self, game: &Game) -> Ordering {
        let parts = self.victory_points_parts(game);
        let other_parts = other.victory_points_parts(game);
        let sum = parts.iter().map(|(_, v)| v).sum::<f32>();
        let other_sum = other_parts.iter().map(|(_, v)| v).sum::<f32>();

        match sum
            .partial_cmp(&other_sum)
            .expect("should be able to compare")
        {
            Less => return Less,
            Equal => (),
            Greater => return Greater,
        }

        for (part, other_part) in parts.iter().zip(other_parts.iter()) {
            match part
                .partial_cmp(other_part)
                .expect("should be able to compare")
            {
                Less => return Less,
                Equal => (),
                Greater => return Greater,
            }
        }
        Equal
    }

    #[must_use]
    pub fn building_cost(&self, game: &Game, building: Building, execute: CostTrigger) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.building_cost,
            &PaymentOptions::resources(self, PaymentReason::Building, BUILDING_COST),
            &building,
            game,
            execute,
        )
    }

    #[must_use]
    pub fn advance_cost(&self, advance: Advance, game: &Game, execute: CostTrigger) -> CostInfo {
        self.trigger_cost_event(
            |e| &e.advance_cost,
            &base_advance_cost(self),
            &advance,
            game,
            execute,
        )
    }

    ///
    /// # Panics
    /// Panics if city does not exist
    #[must_use]
    pub fn get_city(&self, position: Position) -> &City {
        self.try_get_city(position).expect("city should exist")
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
    pub fn get_city_mut(&mut self, position: Position) -> &mut City {
        let position = self
            .cities
            .iter()
            .position(|city| city.position == position)
            .expect("city should exist");
        &mut self.cities[position]
    }

    pub fn take_city(&mut self, position: Position) -> Option<City> {
        Some(
            self.cities.remove(
                self.cities
                    .iter()
                    .position(|city| city.position == position)?,
            ),
        )
    }

    #[must_use]
    pub fn can_raze_city(&self, city_position: Position) -> bool {
        self.cities.len() > 1
            && self
                .try_get_city(city_position)
                .is_some_and(|city| city.size() == 1)
    }

    pub(crate) fn construct(
        &mut self,
        building: Building,
        city_position: Position,
        port_position: Option<Position>,
        activate: bool,
    ) {
        let index = self.index;
        let city = self.get_city_mut(city_position);
        if activate {
            city.activate();
        }
        city.pieces.set_building(building, index);
        if let Some(port_position) = port_position {
            city.port_position = Some(port_position);
        }
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
        value: &PaymentOptions,
        info: &U,
        details: &V,
        trigger: CostTrigger,
    ) -> CostInfo {
        let event = get_event(&self.events.transient).get();
        let mut cost_info = CostInfo::new(self, value.clone());
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
    pub fn custom_action_origin(&self, custom_action_type: &CustomActionType) -> EventOrigin {
        self.custom_actions
            .get(custom_action_type)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "Custom action {} not found for player {}",
                    custom_action_type, self.index
                )
            })
    }
}

pub fn add_unit(player: usize, position: Position, unit_type: UnitType, game: &mut Game) {
    let p = game.player_mut(player);
    let unit = Unit::new(player, position, unit_type, p.next_unit_id);
    p.units.push(unit);
    p.next_unit_id += 1;
    if game.player(player).civilization.is_pirates() {
        for n in position.neighbors() {
            if game.map.is_sea(n) {}
        }
    }
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

pub fn gain_resources(
    game: &mut Game,
    player: usize,
    resources: ResourcePile,
    log: impl Fn(&str, &ResourcePile) -> String,
) {
    game.add_info_log_item(&log(&game.player_name(player), &resources));
    game.player_mut(player).gain_resources(resources);
}

pub(crate) fn can_add_army_unit(p: &Player, position: Position) -> bool {
    p.get_units(position)
        .iter()
        .filter(|u| u.unit_type.is_army_unit())
        .count()
        < STACK_LIMIT
}
