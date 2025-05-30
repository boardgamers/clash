use crate::ability_initializer::AbilityInitializerSetup;
use crate::ability_initializer::{AbilityInitializerBuilder, AbilityListeners};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::player::Player;
use crate::position::Position;
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
}

impl Leader {
    #[must_use]
    pub fn name(&self, game: &Game) -> String {
        game.cache.get_leader(self).name.clone()
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

#[derive(Clone)]
pub struct LeaderAbility {
    pub name: String,
    pub description: String,
    pub listeners: AbilityListeners,
}

impl LeaderAbility {
    #[must_use]
    pub fn builder(name: &str, description: &str) -> LeaderAbilityBuilder {
        LeaderAbilityBuilder::new(name.to_string(), description.to_string())
    }
}

pub struct LeaderAbilityBuilder {
    name: String,
    description: String,
    builder: AbilityInitializerBuilder,
}

impl LeaderAbilityBuilder {
    fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            builder: AbilityInitializerBuilder::new(),
        }
    }

    #[must_use]
    pub fn build(self) -> LeaderAbility {
        LeaderAbility {
            name: self.name,
            description: self.description,
            listeners: self.builder.build(),
        }
    }
}

impl AbilityInitializerSetup for LeaderAbilityBuilder {
    fn builder(&mut self) -> &mut AbilityInitializerBuilder {
        &mut self.builder
    }

    fn get_key(&self) -> EventOrigin {
        EventOrigin::LeaderAbility(self.name.clone())
    }
}

#[must_use]
pub(crate) fn leader_position(player: &Player) -> Position {
    player
        .units
        .iter()
        .find_map(|unit| unit.unit_type.is_leader().then_some(unit.position))
        .expect("unit not found")
}
