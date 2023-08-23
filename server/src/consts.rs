use crate::{city_pieces::AvailableCityPieces, resource_pile::ResourcePile, unit::Units};

pub const MAX_CITY_SIZE: usize = 4;
pub const DICE_ROLL_BUFFER: u32 = 200;
pub const AGES: u32 = 6;
pub const ADVANCE_COST: u32 = 2;
pub const BUILDING_VICTORY_POINTS: f32 = 1.0;
pub const ADVANCE_VICTORY_POINTS: f32 = 0.5;
pub const OBJECTIVE_VICTORY_POINTS: f32 = 2.0;
pub const WONDER_VICTORY_POINTS: f32 = 4.0;
pub const DEFEATED_LEADER_VICTORY_POINTS: f32 = 2.0;
pub const STACK_LIMIT: usize = 4;
pub const SETTLEMENT_LIMIT: u8 = 7;
pub const MOVEMENT_ACTIONS: u32 = 3;
pub const ARMY_MOVEMENT_REQUIRED_ADVANCE: &str = "Tactics";
pub const CITY_PIECE_LIMIT: AvailableCityPieces = AvailableCityPieces {
    academies: 5,
    markets: 5,
    obelisks: 5,
    observatories: 5,
    fortresses: 5,
    ports: 5,
    temples: 5,
};
pub const UNIT_LIMIT: Units = Units {
    settlers: 4,
    infantry: 16,
    ships: 4,
    cavalry: 4,
    elephants: 4,
    leaders: 1,
};
pub const CONSTRUCT_COST: ResourcePile = ResourcePile {
    food: 1,
    wood: 1,
    ore: 1,
    ideas: 0,
    gold: 0,
    mood_tokens: 0,
    culture_tokens: 0,
};
pub const PORT_CHOICES: [ResourcePile; 3] = [
    ResourcePile {
        food: 1,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 0,
        mood_tokens: 0,
        culture_tokens: 0,
    },
    ResourcePile {
        food: 0,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 1,
        mood_tokens: 0,
        culture_tokens: 0,
    },
    ResourcePile {
        food: 0,
        wood: 0,
        ore: 0,
        ideas: 0,
        gold: 0,
        mood_tokens: 1,
        culture_tokens: 0,
    },
];
