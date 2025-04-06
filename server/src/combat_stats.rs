use crate::combat_listeners::CombatResult;
use crate::game::Game;
use crate::position::Position;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units};
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatPlayerStats {
    pub player: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Units::is_empty")]
    pub fighters: Units,
    #[serde(default)]
    #[serde(skip_serializing_if = "Units::is_empty")]
    pub losses: Units,
}

impl CombatPlayerStats {
    #[must_use]
    pub fn new(player: usize, fighters: Units) -> Self {
        Self {
            player,
            fighters,
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
        attacker: CombatPlayerStats,
        defender: CombatPlayerStats,
    ) -> Self {
        Self {
            position,
            battleground,
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
    pub fn player(&self, role: CombatRole) -> &CombatPlayerStats {
        match role {
            CombatRole::Attacker => &self.attacker,
            CombatRole::Defender => &self.defender,
        }
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
    game: &mut Game,
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
    let stats = CombatStats::new(
        defender_position,
        battleground,
        CombatPlayerStats::new(
            attacker,
            attackers
                .iter()
                .map(|id| a.get_unit(*id).unit_type)
                .collect(),
        ),
        CombatPlayerStats::new(
            defender,
            game.player(defender)
                .get_units(defender_position)
                .iter()
                .map(|unit| unit.unit_type)
                .collect(),
        ),
    );
    stats
}
