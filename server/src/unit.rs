use UnitType::*;
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, SubAssign};

use crate::ability_initializer::AbilityInitializerSetup;
use crate::city::is_valid_city_terrain;
use crate::city_pieces::Building;
use crate::combat_roll::COMBAT_DIE_SIDES;
use crate::content::ability::Ability;
use crate::content::civilizations::china::validate_imperial_army;
use crate::content::persistent_events::{KilledUnits, PersistentEventType, UnitsRequest};
use crate::events::EventOrigin;
use crate::explore::is_any_ship;
use crate::game::GameState;
use crate::log::{ActionLogBalance, ActionLogEntry, add_action_log_item};
use crate::movement::{CurrentMove, MovementRestriction};
use crate::player::{Player, remove_unit};
use crate::special_advance::SpecialAdvance;
use crate::{game::Game, leader, position::Position, resource_pile::ResourcePile, unit, utils};

#[derive(Clone)]
pub struct Unit {
    pub player_index: usize,
    pub position: Position,
    pub unit_type: UnitType,
    pub movement_restrictions: Vec<MovementRestriction>,
    pub id: u32,
    pub carrier_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UnitBaseData {
    pub unit_type: UnitType,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub movement_restrictions: Vec<MovementRestriction>,
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UnitData {
    pub position: Position,
    #[serde(flatten)]
    pub data: UnitBaseData,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub carried_units: Vec<UnitBaseData>,
}

impl Unit {
    #[must_use]
    pub fn new(player_index: usize, position: Position, unit_type: UnitType, id: u32) -> Self {
        Self {
            player_index,
            position,
            unit_type,
            movement_restrictions: Vec::new(),
            id,
            carrier_id: None,
        }
    }

    ///
    ///
    /// # Panics
    ///
    /// Panics if unit is at a valid position
    #[must_use]
    pub fn can_found_city(&self, game: &Game) -> bool {
        if !self.is_settler() {
            return false;
        }
        if self.is_transported() {
            return false;
        }
        let player = &game.players[self.player_index];
        if player.try_get_city(self.position).is_some() {
            return false;
        }

        if !is_valid_city_terrain(
            game.map
                .get(self.position)
                .expect("The unit should be at a valid position"),
        ) {
            return false;
        }
        player.is_city_available()
    }

    #[must_use]
    pub fn is_transported(&self) -> bool {
        self.carrier_id.is_some()
    }

    #[must_use]
    pub(crate) fn data(&self, player: &Player) -> UnitData {
        UnitData {
            position: self.position,
            data: UnitBaseData {
                unit_type: self.unit_type,
                movement_restrictions: self.movement_restrictions.clone(),
                id: self.id,
            },
            carried_units: carried_units(self.id, player)
                .iter()
                .map(|id| {
                    let unit = player.get_unit(*id);
                    UnitBaseData {
                        unit_type: unit.unit_type,
                        movement_restrictions: unit.movement_restrictions.clone(),
                        id: unit.id,
                    }
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn from_data(player_index: usize, data: UnitData) -> Vec<Self> {
        let base_data = data.data;
        let unit_id = base_data.id;
        vec![Self {
            player_index,
            position: data.position,
            unit_type: base_data.unit_type,
            movement_restrictions: base_data.movement_restrictions,
            id: unit_id,
            carrier_id: None,
        }]
        .into_iter()
        .chain(data.carried_units.into_iter().map(|c| Self {
            player_index,
            position: data.position,
            unit_type: c.unit_type,
            movement_restrictions: c.movement_restrictions,
            id: c.id,
            carrier_id: Some(unit_id),
        }))
        .collect()
    }

    #[must_use]
    pub fn is_land_based(&self) -> bool {
        self.unit_type.is_land_based()
    }

    #[must_use]
    pub fn is_ship(&self) -> bool {
        self.unit_type.is_ship()
    }

    #[must_use]
    pub fn is_army_unit(&self) -> bool {
        self.unit_type.is_army_unit()
    }

    #[must_use]
    pub fn is_infantry(&self) -> bool {
        self.unit_type == Infantry
    }

    #[must_use]
    pub fn is_settler(&self) -> bool {
        self.unit_type.is_settler()
    }

    #[must_use]
    pub fn is_military(&self) -> bool {
        self.unit_type.is_military()
    }

    #[must_use]
    pub fn is_leader(&self) -> bool {
        self.unit_type.is_leader()
    }
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug, Copy, Ord, PartialOrd)]
pub enum UnitType {
    Settler,
    Infantry,
    Ship,
    Cavalry,
    Elephant,
    Leader(leader::Leader),
}

impl UnitType {
    #[must_use]
    pub fn generic_name(&self) -> &'static str {
        if let Leader(_) = self {
            "leader"
        } else {
            self.non_leader_name()
        }
    }

    ///
    /// # Panics
    ///
    /// Panics if called on a leader unit.
    #[must_use]
    pub fn non_leader_name(&self) -> &'static str {
        match self {
            Settler => "settler",
            Infantry => "infantry",
            Ship => "ship",
            Cavalry => "cavalry",
            Elephant => "elephant",
            Leader(l) => panic!("UnitType::non_leader_name called on a leader unit: {l:?}",),
        }
    }

    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        if let Leader(l) = self {
            return game.cache.get_leader(l).name.clone();
        }
        self.non_leader_name().to_string()
    }

    #[must_use]
    pub fn cost(&self) -> ResourcePile {
        match self {
            Settler | Elephant => ResourcePile::food(2),
            Infantry => ResourcePile::food(1) + ResourcePile::ore(1),
            Ship => ResourcePile::wood(2),
            Cavalry => ResourcePile::food(1) + ResourcePile::wood(1),
            Leader(_) => ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1),
        }
    }

    #[must_use]
    pub fn required_building(&self) -> Option<Building> {
        match self {
            Ship => Some(Building::Port),
            Cavalry | Elephant => Some(Building::Market),
            _ => None,
        }
    }

    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Settler => "A unit that can found cities.".to_string(),
            Ship => "Can fight ships. Can carry 2 land units.".to_string(),
            Infantry => format!(
                "Army unit. Combat abilities: +1 combat values on {}",
                Self::sides(Infantry)
            ),
            Cavalry => format!(
                "Army unit. Combat abilities: +2 combat values on {}",
                Self::sides(Cavalry)
            ),
            Elephant => format!(
                "Army unit. Combat abilities: -1 hit but no combat value on {}",
                Self::sides(Elephant)
            ),
            Leader(_) => format!(
                "Army unit. Combat abilities: Reroll the die until you get a \
             non-leader roll on {}",
                Self::sides(unit::LEADER_UNIT)
            ),
        }
    }

    fn sides(unit_type: UnitType) -> String {
        COMBAT_DIE_SIDES
            .iter()
            .filter(|d| d.bonus == unit_type)
            .map(|d| d.value.to_string())
            .join(", ")
    }

    #[must_use]
    pub fn is_land_based(&self) -> bool {
        !matches!(self, Ship)
    }

    #[must_use]
    pub fn is_ship(&self) -> bool {
        matches!(self, Ship)
    }

    #[must_use]
    pub fn is_army_unit(&self) -> bool {
        matches!(self, Infantry | Cavalry | Elephant | Leader(_))
    }

    #[must_use]
    pub fn is_settler(&self) -> bool {
        matches!(self, Self::Settler)
    }

    #[must_use]
    pub fn is_military(&self) -> bool {
        !self.is_settler()
    }

    #[must_use]
    pub fn is_leader(&self) -> bool {
        matches!(self, Leader(_))
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Units {
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub settlers: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub infantry: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub ships: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub cavalry: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub elephants: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader: Option<leader::Leader>,
}

impl Units {
    #[must_use]
    pub fn new(
        settlers: u8,
        infantry: u8,
        ships: u8,
        cavalry: u8,
        elephants: u8,
        leader: Option<leader::Leader>,
    ) -> Self {
        Self {
            settlers,
            infantry,
            ships,
            cavalry,
            elephants,
            leader,
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0, 0, None)
    }

    /// Type of leader is ignored, it is just a placeholder.
    #[must_use]
    pub fn has_unit(&self, unit: &UnitType) -> bool {
        self.get_amount(unit) > 0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.amount() == 0
    }

    #[must_use]
    pub fn get_amount(&self, unit: &UnitType) -> u8 {
        if matches!(unit, Leader(_)) {
            self.leaders()
        } else {
            self.get(unit)
        }
    }

    #[must_use]
    pub fn get(&self, unit: &UnitType) -> u8 {
        match *unit {
            Settler => self.settlers,
            Infantry => self.infantry,
            Ship => self.ships,
            Cavalry => self.cavalry,
            Elephant => self.elephants,
            Leader(l) => u8::from(self.leader.is_some_and(|leader| leader == l)),
        }
    }

    fn leaders(&self) -> u8 {
        self.leader.map_or(0, |_| 1)
    }

    #[must_use]
    pub fn has_leader(&self) -> bool {
        self.leader.is_some()
    }

    #[must_use]
    pub fn amount(&self) -> u8 {
        self.settlers + self.infantry + self.ships + self.cavalry + self.elephants + self.leaders()
    }

    #[must_use]
    pub fn to_vec(self) -> Vec<UnitType> {
        self.into_iter()
            .flat_map(|(u, c)| std::iter::repeat_n(u, c as usize))
            .collect()
    }

    ///
    /// # Panics
    ///
    /// Panics if `game` is `None` and the units contain a leader.
    #[must_use]
    pub fn to_string(&self, game: Option<&Game>) -> String {
        let mut unit_types = Vec::new();
        if self.settlers > 0 {
            unit_types.push(format!(
                "{} {}",
                self.settlers,
                if self.settlers == 1 {
                    "settler"
                } else {
                    "settlers"
                }
            ));
        }
        if self.infantry > 0 {
            unit_types.push(format!("{} infantry", self.infantry,));
        }
        if self.ships > 0 {
            unit_types.push(format!(
                "{} {}",
                self.ships,
                if self.ships == 1 { "ship" } else { "ships" }
            ));
        }
        if self.cavalry > 0 {
            unit_types.push(format!("{} cavalry", self.cavalry,));
        }
        if self.elephants > 0 {
            unit_types.push(format!(
                "{} {}",
                self.elephants,
                if self.elephants == 1 {
                    "elephant"
                } else {
                    "elephants"
                }
            ));
        }
        if let Some(l) = self.leader {
            if let Some(game) = game {
                unit_types.push(l.name(game));
            } else {
                panic!("game missing for leader")
            }
        }
        utils::format_and(&unit_types, "no units")
    }
}

impl Default for Units {
    fn default() -> Self {
        Self::empty()
    }
}

impl AddAssign<&UnitType> for Units {
    fn add_assign(&mut self, rhs: &UnitType) {
        match *rhs {
            Settler => self.settlers += 1,
            Infantry => self.infantry += 1,
            Ship => self.ships += 1,
            Cavalry => self.cavalry += 1,
            Elephant => self.elephants += 1,
            Leader(l) => self.leader = Some(l),
        }
    }
}

impl SubAssign<&UnitType> for Units {
    fn sub_assign(&mut self, rhs: &UnitType) {
        match *rhs {
            Settler => self.settlers -= 1,
            Infantry => self.infantry -= 1,
            Ship => self.ships -= 1,
            Cavalry => self.cavalry -= 1,
            Elephant => self.elephants -= 1,
            Leader(_) => self.leader = None,
        }
    }
}

impl FromIterator<UnitType> for Units {
    fn from_iter<T: IntoIterator<Item = UnitType>>(iter: T) -> Self {
        let mut units = Self::empty();
        for unit in iter {
            units += &unit;
        }
        units
    }
}

impl IntoIterator for Units {
    type Item = (UnitType, u8);
    type IntoIter = UnitsIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        UnitsIntoIterator {
            units: self,
            index: 0,
        }
    }
}

pub struct UnitsIntoIterator {
    units: Units,
    index: u8,
}

impl Iterator for UnitsIntoIterator {
    type Item = (UnitType, u8);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        let u = &self.units;
        match index {
            0 => Some((Settler, u.settlers)),
            1 => Some((Infantry, u.infantry)),
            2 => Some((Ship, u.ships)),
            3 => Some((Cavalry, u.cavalry)),
            4 => Some((Elephant, u.elephants)),
            5 => Some((
                self.units.leader.map_or(unit::LEADER_UNIT, Leader),
                u.leaders(),
            )),
            _ => None,
        }
    }
}

#[must_use]
pub fn carried_units(carrier: u32, player: &Player) -> Vec<u32> {
    player
        .units
        .iter()
        .filter(|u| u.carrier_id == Some(carrier))
        .map(|u| u.id)
        .collect()
}

pub(crate) fn get_current_move(
    game: &Game,
    units: &[u32],
    starting: Position,
    destination: Position,
    embark_carrier_id: Option<u32>,
) -> CurrentMove {
    if embark_carrier_id.is_some() {
        CurrentMove::Embark {
            source: starting,
            destination,
        }
    } else if is_any_ship(game, game.current_player_index, units) {
        CurrentMove::Fleet {
            units: units.iter().sorted().copied().collect(),
        }
    } else {
        CurrentMove::None
    }
}

pub(crate) fn kill_units(
    game: &mut Game,
    unit_ids: &[u32],
    player_index: usize,
    killer: Option<usize>,
    origin: &EventOrigin,
) {
    let pos = game
        .player(player_index)
        .get_unit(*unit_ids.first().expect("no units"))
        .position;
    let killed_units = KilledUnits::new(pos, killer);
    kill_units_without_event(game, unit_ids, player_index, &killed_units, origin);
    units_killed(game, player_index, killed_units);
}

pub(crate) fn kill_units_without_event(
    game: &mut Game,
    unit_ids: &[u32],
    player: usize,
    killed_units: &KilledUnits,
    origin: &EventOrigin,
) {
    let p = game.player(player);
    let units = unit_ids
        .iter()
        .map(|id| p.get_unit(*id).unit_type)
        .collect::<Units>();
    game.log(
        player,
        origin,
        &format!(
            "Lost {} at {}",
            units.to_string(Some(game)),
            killed_units.position
        ),
    );
    add_action_log_item(
        game,
        player,
        ActionLogEntry::units(units, ActionLogBalance::Loss),
        origin.clone(),
        vec![],
    );

    for unit in unit_ids {
        kill_unit(game, *unit, player, killed_units.killer);
    }
}

pub(crate) fn units_killed(game: &mut Game, player_index: usize, killed_units: KilledUnits) {
    match game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.units_killed,
        killed_units,
        PersistentEventType::UnitsKilled,
    ) {
        None => (),
        Some(killed_units) => save_carried_units(game, player_index, killed_units.position),
    }
}

fn kill_unit(game: &mut Game, unit_id: u32, player_index: usize, killer: Option<usize>) {
    let unit = remove_unit(player_index, unit_id, game);
    if let Leader(leader) = unit.unit_type {
        Player::with_leader(leader, game, player_index, |game, leader| {
            leader.listeners.deinit(game, player_index);
        });
        if let Some(killer) = killer {
            game.players[killer].captured_leaders.push(leader);
        }
    }
    if let GameState::Movement(m) = &mut game.state {
        if let CurrentMove::Fleet { units } = &mut m.current_move {
            units.retain(|&id| id != unit_id);
        }
    }
}

fn save_carried_units(game: &mut Game, player: usize, pos: Position) {
    let mut survivors = game
        .player(player)
        .get_units(pos)
        .iter()
        .filter(|u| {
            u.carrier_id
                .is_some_and(|id| game.player(player).try_get_unit(id).is_none())
        })
        .map(|u| (u.id))
        .collect_vec();

    if survivors.is_empty() {
        return;
    }

    game.information_revealed(); // strange bug when redoing this

    let mut embark = vec![];

    game.player(player)
        .get_units(pos)
        .iter()
        .filter(|u| u.is_ship())
        .map(|u| {
            let p = game.player(player);
            let mut capacity = ship_capacity(p) - carried_units(u.id, p).len() as u8;
            while capacity > 0 {
                capacity -= 1;
                if let Some(survivor) = survivors.pop() {
                    embark.push((survivor, u.id));
                }
            }
        })
        .collect_vec();

    for (survivor, carrier) in embark {
        game.player_mut(player).get_unit_mut(survivor).carrier_id = Some(carrier);
    }
}

pub(crate) fn choose_carried_units_to_remove() -> Ability {
    Ability::builder(
        "Choose Casualties (carried units)",
        "Choose which carried units to remove.",
    )
    .add_units_request(
        |event| &mut event.units_killed,
        0,
        |game, player, k| {
            let p = player.get(game);
            let pos = k.position;
            if game.map.is_land(pos) {
                return None;
            }
            let carried: Vec<u32> = p
                .get_units(pos)
                .into_iter()
                .filter(|u| u.carrier_id.is_some())
                .map(|u| u.id)
                .collect();
            let capacity =
                p.get_units(pos).iter().filter(|u| u.is_ship()).count() * ship_capacity(p) as usize;
            let to_kill = carried.len().saturating_sub(capacity) as u8;

            Some(UnitsRequest::new(
                player.index,
                carried,
                to_kill..=to_kill,
                "Choose which carried units to remove",
            ))
        },
        |game, s, e| {
            if !s.choice.is_empty() {
                let p = game.player(s.player_index);
                let units = s
                    .choice
                    .iter()
                    .map(|id| p.get_unit(*id).unit_type)
                    .collect_vec();

                for event in &mut game.events {
                    if let PersistentEventType::CombatRoundEnd(e) = &mut event.event_type {
                        let c = &mut e.combat;

                        c.stats
                            .player_mut(c.role(s.player_index))
                            .add_losses(&units);
                    }
                }

                s.log(
                    game,
                    &format!(
                        "Killed carried units: {}",
                        units.into_iter().collect::<Units>().to_string(Some(game))
                    ),
                );
            }
            kill_units_without_event(game, &s.choice, s.player_index, e, &s.origin);
        },
    )
    .build()
}

pub fn set_unit_position(player: usize, unit_id: u32, position: Position, game: &mut Game) {
    game.player_mut(player).get_unit_mut(unit_id).position = position;
}

#[must_use]
pub fn get_units_to_replace(available: &Units, new_units: &Units) -> Units {
    let mut units_to_replace = Units::empty();
    for (unit_type, count) in available.clone() {
        for _ in 0..new_units.get_amount(&unit_type).saturating_sub(count) {
            units_to_replace += &unit_type;
        }
    }
    units_to_replace
}

// ignore the concrete leader here, it is just a placeholder
pub const LEADER: leader::Leader = leader::Leader::Alexander;

pub const LEADER_UNIT: UnitType = Leader(LEADER);

///
/// Validates the selection of cards in the hand.
///
/// # Returns
///
/// Card names to show in the UI - if possible.
///
/// # Errors
///
/// If the selection is invalid, an error message is returned.
pub fn validate_units_selection(units: &[u32], game: &Game, p: &Player) -> Result<(), String> {
    let Some(h) = &game.current_event().player.handler.as_ref() else {
        return Err("no selection handler".to_string());
    };
    validate_units_selection_for_origin(units, p, &h.origin)
}

pub(crate) fn validate_units_selection_for_origin(
    units: &[u32],
    p: &Player,
    o: &EventOrigin,
) -> Result<(), String> {
    match o {
        EventOrigin::Ability(b) if b == "Imperial Army" => validate_imperial_army(units, p),
        _ => Ok(()),
    }
}

pub(crate) fn ship_capacity(p: &Player) -> u8 {
    if p.has_special_advance(SpecialAdvance::Longships) {
        3
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use crate::unit::UnitType::*;
    use crate::unit::{Units, get_units_to_replace};
    use crate::{leader, unit};

    #[test]
    fn into_iter() {
        let units = Units::new(0, 1, 0, 2, 1, Some(leader::Leader::Sulla));
        assert_eq!(
            units.into_iter().collect::<Vec<_>>(),
            vec![
                (Settler, 0),
                (Infantry, 1),
                (Ship, 0),
                (Cavalry, 2),
                (Elephant, 1),
                (Leader(leader::Leader::Sulla), 1),
            ]
        );
    }

    #[test]
    fn test_get_units_to_replace() {
        let available = Units::new(0, 1, 0, 2, 1, Some(unit::LEADER));
        let new_units = Units::new(0, 2, 0, 1, 1, Some(leader::Leader::Sulla));
        assert_eq!(
            get_units_to_replace(&available, &new_units),
            Units::new(0, 1, 0, 0, 0, None)
        );
    }
}
