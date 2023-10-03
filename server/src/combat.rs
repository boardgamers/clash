use crate::game::Game;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader};
use crate::unit::Units;

pub struct CombatRolls {
    pub combat_value: u8,
    pub hit_cancels: u8,
}

// Roll translation
// 0 = 1 leader
// 1 = 1 leader
// 2 = 2 cavalry
// 3 = 2 elephant
// 4 = 3 elephant
// 5 = 3 infantry
// 6 = 4 cavalry
// 7 = 4 elephant
// 8 = 5 cavalry
// 9 = 5 infantry
// 10= 6 infantry
// 11= 6 infantry

pub(crate) fn roll(game: &mut Game, player_index: usize, units: &Vec<u32>) -> CombatRolls {
    let mut dice_rolls = 0;
    let mut unit_types = Units::empty();
    for unit in units {
        let unit = &game.players[player_index]
            .get_unit(*unit)
            .expect("player should have all units")
            .unit_type;
        if unit.is_settler() {
            continue;
        }
        dice_rolls += 1;
        unit_types += unit;
    }

    let mut rolls = CombatRolls {
        combat_value: 0,
        hit_cancels: 0,
    };
    rolls.combat_value = 0;
    rolls.hit_cancels = 0;
    for _ in 0..dice_rolls {
        let dice_roll = dice_roll_with_leader_reroll(game, &mut unit_types);
        let value = dice_value(dice_roll);
        rolls.combat_value += value;
        match dice_roll {
            5 | 9 | 10 | 11 => {
                if unit_types.has_unit(&Infantry) {
                    rolls.combat_value += 1;
                    unit_types -= &Infantry;
                }
            }
            2 | 6 | 8 => {
                if unit_types.has_unit(&Cavalry) {
                    rolls.combat_value += 2;
                    unit_types -= &Cavalry;
                }
            }
            3 | 4 | 7 => {
                if unit_types.has_unit(&Elephant) {
                    rolls.hit_cancels += 1;
                    rolls.combat_value -= value;
                    unit_types -= &Elephant;
                }
            }
            _ => (),
        }
    }
    rolls
}

fn dice_roll_with_leader_reroll(game: &mut Game, unit_types: &mut Units) -> u8 {
    loop {
        let roll = game.get_next_dice_roll();

        if roll > 2 || !unit_types.has_unit(&Leader) {
            return roll;
        }
        *unit_types -= &Leader;
    }
}

#[must_use]
pub fn dice_value(roll: u8) -> u8 {
    roll / 2 + 1
}
