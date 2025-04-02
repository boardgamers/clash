use UnitType::*;
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::iter;
use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};

use crate::ability_initializer::AbilityInitializerSetup;
use crate::consts::SHIP_CAPACITY;
use crate::content::builtin::Builtin;
use crate::content::persistent_events::{KilledUnits, PersistentEventType, UnitsRequest};
use crate::explore::is_any_ship;
use crate::game::GameState;
use crate::movement::{CurrentMove, MovementRestriction};
use crate::player::Player;
use crate::{game::Game, map::Terrain::*, position::Position, resource_pile::ResourcePile, utils};

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
        if !self.unit_type.is_settler() {
            return false;
        }
        if self.is_transported() {
            return false;
        }
        let player = &game.players[self.player_index];
        if player.try_get_city(self.position).is_some() {
            return false;
        }
        if matches!(
            game.map
                .get(self.position)
                .expect("The unit should be at a valid position"),
            Barren | Exhausted(_)
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
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq, Debug, Copy)]
pub enum UnitType {
    Settler,
    Infantry,
    Ship,
    Cavalry,
    Elephant,
    Leader,
}

impl UnitType {
    #[must_use]
    pub fn get_all() -> Vec<Self> {
        vec![Settler, Infantry, Ship, Cavalry, Elephant, Leader]
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Settler => "settler",
            Infantry => "infantry",
            Ship => "ship",
            Cavalry => "cavalry",
            Elephant => "elephant",
            Leader => "leader",
        }
    }

    #[must_use]
    pub fn cost(&self) -> ResourcePile {
        match self {
            Settler | Elephant => ResourcePile::food(2),
            Infantry => ResourcePile::food(1) + ResourcePile::ore(1),
            Ship => ResourcePile::wood(2),
            Cavalry => ResourcePile::food(1) + ResourcePile::wood(1),
            Leader => ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1),
        }
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
        matches!(self, Infantry | Cavalry | Elephant | Leader)
    }

    /// Returns `true` if the unit type is [`Settler`].
    ///
    /// [`Settler`]: UnitType::Settler
    #[must_use]
    pub fn is_settler(&self) -> bool {
        matches!(self, Self::Settler)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
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
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub leaders: u8,
}

impl Units {
    #[must_use]
    pub fn new(
        settlers: u8,
        infantry: u8,
        ships: u8,
        cavalry: u8,
        elephants: u8,
        leaders: u8,
    ) -> Self {
        Self {
            settlers,
            infantry,
            ships,
            cavalry,
            elephants,
            leaders,
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0, 0, 0)
    }

    #[must_use]
    pub fn has_unit(&self, unit: &UnitType) -> bool {
        self.get(unit) > 0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clone().to_vec().is_empty()
    }

    #[must_use]
    pub fn get(&self, unit: &UnitType) -> u8 {
        match *unit {
            Settler => self.settlers,
            Infantry => self.infantry,
            Ship => self.ships,
            Cavalry => self.cavalry,
            Elephant => self.elephants,
            Leader => self.leaders,
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, unit: &UnitType) -> &mut u8 {
        match *unit {
            Settler => &mut self.settlers,
            Infantry => &mut self.infantry,
            Ship => &mut self.ships,
            Cavalry => &mut self.cavalry,
            Elephant => &mut self.elephants,
            Leader => &mut self.leaders,
        }
    }

    #[must_use]
    pub fn to_vec(self) -> Vec<UnitType> {
        self.into_iter()
            .flat_map(|(u, c)| iter::repeat(u).take(c as usize))
            .collect()
    }

    #[must_use]
    pub fn get_units_to_replace(&self, new_units: &Units) -> Units {
        let mut units_to_replace = Units::empty();
        for (unit_type, count) in self.clone() {
            let new_count = new_units.get(&unit_type) as i8;
            let replace = new_count - count as i8;
            if replace > 0 {
                *units_to_replace.get_mut(&unit_type) += replace as u8;
            }
        }
        units_to_replace
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
            Leader => self.leaders += 1,
        };
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
            Leader => self.leaders -= 1,
        };
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
            5 => Some((Leader, u.leaders)),
            _ => None,
        }
    }
}

impl Display for Units {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        if self.leaders > 0 {
            unit_types.push(if self.leaders == 1 {
                String::from("a leader")
            } else {
                format!("{} leaders", self.leaders)
            });
        }
        write!(f, "{}", utils::format_and(&unit_types, "no units"))
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
) {
    let pos = game
        .player(player_index)
        .get_unit(*unit_ids.first().expect("no units"))
        .position;
    kill_units_without_event(game, unit_ids, player_index, killer);
    units_killed(game, player_index, KilledUnits::new(pos, killer));
}

pub(crate) fn kill_units_without_event(
    game: &mut Game,
    unit_ids: &[u32],
    player_index: usize,
    killer: Option<usize>,
) {
    for unit in unit_ids {
        kill_unit(game, *unit, player_index, killer);
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
    let unit = game.players[player_index].remove_unit(unit_id);
    if matches!(unit.unit_type, UnitType::Leader) {
        let leader = game.players[player_index]
            .active_leader
            .take()
            .expect("A player should have an active leader when having a leader unit");
        Player::with_leader(&leader, game, player_index, |game, leader| {
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

    game.lock_undo(); // strange bug when redoing this

    let mut embark = vec![];

    game.player(player)
        .get_units(pos)
        .iter()
        .filter(|u| u.unit_type.is_ship())
        .map(|u| {
            let mut capacity = SHIP_CAPACITY - carried_units(u.id, game.player(player)).len() as u8;
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

pub(crate) fn choose_carried_units_casualties() -> Builtin {
    Builtin::builder(
        "Choose Casualties (carried units)",
        "Choose which carried units to remove.",
    )
    .add_units_request(
        |event| &mut event.units_killed,
        0,
        |game, player, k| {
            let p = game.player(player);
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
            let capacity = p
                .get_units(pos)
                .iter()
                .filter(|u| u.unit_type.is_ship())
                .count()
                * SHIP_CAPACITY as usize;
            let to_kill = carried.len().saturating_sub(capacity) as u8;

            Some(UnitsRequest::new(
                player,
                carried,
                to_kill..=to_kill,
                "Choose which carried units to remove",
            ))
        },
        |game, units, e| {
            if !units.choice.is_empty() {
                let p = game.player(units.player_index);
                game.add_info_log_item(&format!(
                    "{} killed carried units: {}",
                    units.player_name,
                    units
                        .choice
                        .iter()
                        .map(|id| p.get_unit(*id).unit_type)
                        .collect::<Units>()
                ));
            }
            kill_units_without_event(game, &units.choice, units.player_index, e.killer);
        },
    )
    .build()
}

#[cfg(test)]
mod tests {
    use crate::unit::UnitType::*;
    use crate::unit::Units;

    #[test]
    fn into_iter() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        assert_eq!(units.into_iter().collect::<Vec<_>>(), vec![
            (Settler, 0),
            (Infantry, 1),
            (Ship, 0),
            (Cavalry, 2),
            (Elephant, 1),
            (Leader, 1),
        ]);
    }

    #[test]
    fn to_vec() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        assert_eq!(units.to_vec(), vec![
            Infantry, Cavalry, Cavalry, Elephant, Leader
        ]);
    }

    #[test]
    fn get_units_to_replace() {
        let units = Units::new(0, 1, 0, 2, 1, 1);
        let new_units = Units::new(0, 2, 0, 1, 1, 1);
        assert_eq!(
            units.get_units_to_replace(&new_units),
            Units::new(0, 1, 0, 0, 0, 0)
        );
    }
}
