use crate::combat::can_remove_after_combat;
use crate::combat_listeners::CombatResult;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units};
use crate::utils;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatPlayerStats {
    pub player: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Units::is_empty")]
    pub present: Units,
    #[serde(default)]
    #[serde(skip_serializing_if = "Units::is_empty")]
    pub losses: Units,
}

impl CombatPlayerStats {
    #[must_use]
    pub fn new(player: usize, present: Units) -> Self {
        Self {
            player,
            present,
            losses: Units::empty(),
        }
    }

    pub fn add_losses(&mut self, units: &[UnitType]) {
        let mut losses = mem::replace(&mut self.losses, Units::empty());
        for t in units {
            losses += t;
        }
        self.losses = losses;
    }

    pub fn fighters(&self, battleground: Battleground) -> Units {
        filter_fighters(battleground, &self.present)
    }

    pub fn fighter_losses(&self, battleground: Battleground) -> Units {
        filter_fighters(battleground, &self.losses)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum Battleground {
    Land,
    Sea,
    City,
    CityWithFortress,
}

impl Battleground {
    #[must_use]
    pub(crate) fn is_land(self) -> bool {
        !matches!(self, Battleground::Sea)
    }

    #[must_use]
    pub fn is_city(self) -> bool {
        matches!(self, Battleground::City | Battleground::CityWithFortress)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatStats {
    pub position: Position,
    pub battleground: Battleground,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub disembarked: bool,
    pub attacker: CombatPlayerStats,
    pub defender: CombatPlayerStats,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CombatResult>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub claimed_action_cards: Vec<u8>,
}

impl CombatStats {
    #[must_use]
    pub fn new(
        position: Position,
        battleground: Battleground,
        disembarked: bool,
        attacker: CombatPlayerStats,
        defender: CombatPlayerStats,
    ) -> Self {
        Self {
            position,
            battleground,
            disembarked,
            attacker,
            defender,
            result: None,
            claimed_action_cards: Vec::new(),
        }
    }

    #[must_use]
    pub fn player_mut(&mut self, role: CombatRole) -> &mut CombatPlayerStats {
        match role {
            CombatRole::Attacker => &mut self.attacker,
            CombatRole::Defender => &mut self.defender,
        }
    }

    #[must_use]
    pub fn player(&self, player: usize) -> &CombatPlayerStats {
        match self.role(player) {
            CombatRole::Attacker => &self.attacker,
            CombatRole::Defender => &self.defender,
        }
    }

    #[must_use]
    pub fn opponent(&self, player: usize) -> &CombatPlayerStats {
        match self.opponent_role(player) {
            CombatRole::Attacker => &self.attacker,
            CombatRole::Defender => &self.defender,
        }
    }

    #[must_use]
    pub fn opponent_is_human(&self, player: usize, game: &Game) -> bool {
        game.player(self.opponent(player).player).is_human()
    }

    #[must_use]
    pub fn role(&self, player: usize) -> CombatRole {
        if player == self.attacker.player {
            CombatRole::Attacker
        } else {
            CombatRole::Defender
        }
    }

    #[must_use]
    pub fn opponent_role(&self, player: usize) -> CombatRole {
        if player == self.attacker.player {
            CombatRole::Defender
        } else {
            CombatRole::Attacker
        }
    }

    #[must_use]
    pub fn winner(&self) -> Option<CombatRole> {
        self.result.as_ref().and_then(|result| match result {
            CombatResult::AttackerWins => Some(CombatRole::Attacker),
            CombatResult::DefenderWins => Some(CombatRole::Defender),
            CombatResult::Draw => None,
        })
    }

    #[must_use]
    pub fn is_winner(&self, player: usize) -> bool {
        Some(self.role(player)) == self.winner()
    }
}

pub(crate) fn new_combat_stats(
    game: &Game,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attackers: &[u32],
) -> CombatStats {
    let city = game.try_get_any_city(defender_position);

    let battleground = if let Some(city) = city {
        if city.pieces.fortress.is_some() {
            Battleground::CityWithFortress
        } else {
            Battleground::City
        }
    } else if game.map.is_sea(defender_position) {
        Battleground::Sea
    } else {
        Battleground::Land
    };

    let a = game.player(attacker);
    let d = game.player(defender);
    let stats = CombatStats::new(
        defender_position,
        battleground,
        battleground.is_land() && game.map.is_sea(a.get_unit(attackers[0]).position),
        CombatPlayerStats::new(attacker, to_units(attackers, a)),
        CombatPlayerStats::new(
            defender,
            d.get_units(defender_position)
                .iter()
                .map(|unit| unit.unit_type)
                .collect(),
        ),
    );
    stats
}

fn to_units(units: &[u32], p: &Player) -> Units {
    units.iter().map(|id| p.get_unit(*id).unit_type).collect()
}

#[must_use]
pub(crate) fn active_attackers(
    game: &Game,
    attacker: usize,
    attackers: &[u32],
    defender_position: Position,
) -> Vec<u32> {
    let player = &game.players[attacker];

    let on_water = game.map.is_sea(defender_position);
    attackers
        .iter()
        .copied()
        .filter(|u| can_remove_after_combat(on_water, &player.get_unit(*u).unit_type))
        .collect_vec()
}

#[must_use]
pub fn active_defenders(game: &Game, defender: usize, defender_position: Position) -> Vec<u32> {
    let p = &game.players[defender];
    let on_water = game.map.is_sea(defender_position);
    p.get_units(defender_position)
        .into_iter()
        .filter(|u| can_remove_after_combat(on_water, &u.unit_type))
        .map(|u| u.id)
        .collect_vec()
}

fn filter_fighters(battleground: Battleground, units: &Units) -> Units {
    units
        .clone()
        .to_vec()
        .into_iter()
        .filter(|u| {
            if battleground.is_land() {
                u.is_army_unit()
            } else {
                u.is_ship()
            }
        })
        .collect()
}
