use crate::game::Game;
use crate::leader_ability::LeaderAbility;
use crate::player::Player;
use crate::position::Position;
use crate::unit::UnitType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
pub enum Leader {
    // Test
    Kirk,
    Janeway,
    Spock,
    BorgQueen,
    SevenOfNine,
    Picard,
    Khan,
    Kahless,
    Worf,
    Sela,
    Narek,
    Tomalak,

    // for Egypt later
    Kleopatra,

    // Rome
    Augustus,
    Caesar,
    Sulla,
    // Greece
    Alexander,
    Leonidas,
    Pericles,
    // China
    Qin,
    Wu,
    SunTzu,
}

impl Leader {
    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        game.cache.get_leader(self).name.clone()
    }

    #[must_use]
    pub fn unit_type(&self) -> UnitType {
        UnitType::Leader(*self)
    }
}

#[derive(Clone)]
pub struct LeaderInfo {
    pub leader: Leader,
    pub name: String,
    pub abilities: Vec<LeaderAbility>,
}

impl LeaderInfo {
    #[must_use]
    pub fn new(
        leader: Leader,
        name: &str,
        first_ability: LeaderAbility,
        second_ability: LeaderAbility,
    ) -> LeaderInfo {
        Self {
            leader,
            name: name.to_string(),
            abilities: vec![first_ability, second_ability],
        }
    }
}

#[must_use]
pub(crate) fn leader_position(player: &Player) -> Position {
    player
        .units
        .iter()
        .find_map(|unit| unit.is_leader().then_some(unit.position))
        .expect("unit not found")
}
