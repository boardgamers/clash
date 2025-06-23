use crate::advance::Advance;
use crate::{resource_pile::ResourcePile, unit, unit::Units};

pub const JSON_SCHEMA_VERSION: u16 = 2;
pub const MAX_CITY_PIECES: usize = 4; // i.e. city size 5
pub const AGES: u32 = 6;
pub const ADVANCE_COST: u8 = 2;
pub const BUILDING_VICTORY_POINTS: f32 = 1.0;
pub const ADVANCE_VICTORY_POINTS: f32 = 0.5;
pub const OBJECTIVE_VICTORY_POINTS: f32 = 2.0;
pub const WONDER_VICTORY_POINTS: u8 = 4;
pub const CAPTURED_LEADER_VICTORY_POINTS: f32 = 2.0;
pub const STACK_LIMIT: usize = 4;
pub const CITY_LIMIT: u8 = 7;
pub const MOVEMENT_ACTIONS: u32 = 3;
pub const ARMY_MOVEMENT_REQUIRED_ADVANCE: Advance = Advance::Tactics;
pub const CITY_PIECE_LIMIT: usize = 5;
pub const ACTIONS: u32 = 3;
pub const NON_HUMAN_PLAYERS: usize = 2; // pirates, barbarians
pub const MAX_HUMAN_PLAYERS: usize = 4; // pirates, barbarians
pub const INFLUENCE_MIN_ROLL: u8 = 5;

pub const UNIT_LIMIT: Units = Units {
    settlers: 4,
    infantry: 16,
    ships: 4,
    cavalry: 4,
    elephants: 4,
    leader: Some(unit::LEADER),
};
pub const UNIT_LIMIT_BARBARIANS: Units = Units {
    settlers: 0,
    infantry: 20,
    ships: 0,
    cavalry: 4,
    elephants: 4,
    leader: None,
};
pub const UNIT_LIMIT_PIRATES: Units = Units {
    settlers: 0,
    infantry: 0,
    ships: 4,
    cavalry: 0,
    elephants: 0,
    leader: None,
};
pub const BUILDING_COST: ResourcePile = ResourcePile {
    food: 1,
    wood: 1,
    ore: 1,
    ideas: 0,
    gold: 0,
    mood_tokens: 0,
    culture_tokens: 0,
};
