use crate::city::MoodState;
use crate::combat::can_remove_after_combat;
use crate::combat_listeners::CombatResult;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatPlayerStats {
    pub position: Position,
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
    pub fn new(player: usize, present: Units, position: Position) -> Self {
        Self {
            player,
            present,
            losses: Units::empty(),
            position,
        }
    }

    pub fn add_losses(&mut self, units: &[UnitType]) {
        let mut losses = std::mem::take(&mut self.losses);
        for t in units {
            losses += t;
        }
        self.losses = losses;
    }

    #[must_use]
    pub fn fighters(&self, battleground: Battleground) -> Units {
        filter_fighters(battleground, &self.present)
    }

    #[must_use]
    pub fn fighter_losses(&self, battleground: Battleground) -> Units {
        filter_fighters(battleground, &self.losses)
    }

    #[must_use]
    pub fn survived_leader(&self) -> bool {
        self.present.has_leader() && !self.losses.has_leader()
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
    pub round: u32, //starts with one,
    pub battleground: Battleground,
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub disembarked: bool,
    pub attacker: CombatPlayerStats,
    pub defender: CombatPlayerStats,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CombatResult>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub claimed_action_cards: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_card: Option<u8>, // for "Teach Us Now" action
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_mood: Option<MoodState>, // before capture
}

impl CombatStats {
    #[must_use]
    pub fn new(
        battleground: Battleground,
        disembarked: bool,
        attacker: CombatPlayerStats,
        defender: CombatPlayerStats,
        result: Option<CombatResult>,
        city_mood: Option<MoodState>,
    ) -> Self {
        Self {
            battleground,
            disembarked,
            attacker,
            defender,
            result,
            claimed_action_cards: Vec::new(),
            selected_card: None,
            round: 1,
            city_mood,
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
        self.opponent_player(player, game).is_human()
    }

    #[must_use]
    pub fn opponent_player<'a>(&self, player: usize, game: &'a Game) -> &'a Player {
        game.player(self.opponent(player).player)
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
    pub fn is_attacker(&self, player: usize) -> bool {
        self.role(player) == CombatRole::Attacker
    }

    #[must_use]
    pub fn is_defender(&self, player: usize) -> bool {
        self.role(player) == CombatRole::Defender
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

    #[must_use]
    pub fn is_loser(&self, player: usize) -> bool {
        Some(self.opponent_role(player)) == self.winner()
    }

    #[must_use]
    pub fn is_battle(&self) -> bool {
        // defender can win with a fortress or great wall
        self.defender.present.amount() > 0 || self.winner() == Some(CombatRole::Defender)
    }

    #[must_use]
    pub fn captured_city(&self, player: usize) -> Option<MoodState> {
        (self.is_attacker(player) && self.is_winner(player))
            .then(|| self.city_mood.clone())
            .flatten()
    }
}

pub(crate) fn new_combat_stats(
    game: &Game,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attackers: &[u32],
    result: Option<CombatResult>,
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
    let attacker_position = a.get_unit(attackers[0]).position;
    assert_ne!(defender_position, attacker_position);

    CombatStats::new(
        battleground,
        battleground.is_land() && game.map.is_sea(attacker_position),
        CombatPlayerStats::new(attacker, to_units(attackers, a), attacker_position),
        CombatPlayerStats::new(
            defender,
            d.get_units(defender_position)
                .iter()
                .map(|unit| unit.unit_type)
                .collect(),
            defender_position,
        ),
        result,
        city.map(|c| c.mood_state.clone()),
    )
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
