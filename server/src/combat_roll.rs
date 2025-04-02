use crate::combat::Combat;
use crate::combat_listeners::CombatStrength;
use crate::game::Game;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader};
use crate::unit::{UnitType, Units};

pub(crate) struct CombatStats {
    player: usize,
    opponent_str: String,
    pub(crate) fighters: u8,
    log_str: String,
    combat_value: u8,
    hit_cancels: u8,
    pub(crate) hits: u8,
    strength: CombatStrength,
}

impl CombatStats {
    pub(crate) fn roll(
        player: usize,
        c: &Combat,
        game: &mut Game,
        strength: CombatStrength,
    ) -> CombatStats {
        let fighting = c.fighting_units(game, player);
        let mut log = vec![];
        let rolls = roll(
            game,
            player,
            &fighting,
            strength.extra_dies,
            strength.extra_combat_value,
            strength.deny_combat_abilities,
            &mut log,
        );
        let log_str = roll_log_str(&log);
        let combat_value = rolls.combat_value as u8;
        let hit_cancels = rolls.hit_cancels + strength.hit_cancels;

        let opponent_str = if c.defender == player {
            "attacking"
        } else {
            "defending"
        }
        .to_string();

        CombatStats {
            opponent_str,
            strength,
            player,
            log_str,
            combat_value,
            hit_cancels,
            hits: 0,
            fighters: fighting.len() as u8,
        }
    }

    pub(crate) fn determine_hits(&mut self, opponent: &CombatStats, game: &mut Game) {
        self.hits = (self.combat_value / 5)
            .saturating_sub(opponent.hit_cancels)
            .min(opponent.fighters);

        let name = game.player_name(self.player);
        game.add_info_log_item(&format!(
            "{name} rolled {} for combined combat value of {} and gets {} hits \
            against {} units.",
            self.log_str, self.combat_value, self.hits, self.opponent_str,
        ));

        if !self.strength.roll_log.is_empty() {
            game.add_info_log_item(&format!(
                "{name} used the following combat modifiers: {}",
                self.strength.roll_log.join(", ")
            ));
        }
    }
}

fn roll_log_str(log: &[String]) -> String {
    if log.is_empty() {
        return String::from("no dice");
    }
    log.join(", ")
}

struct CombatRolls {
    pub combat_value: i8,
    pub hit_cancels: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct CombatDieRoll {
    pub value: u8,
    pub bonus: UnitType,
}

impl CombatDieRoll {
    #[must_use]
    pub const fn new(value: u8, bonus: UnitType) -> Self {
        Self { value, bonus }
    }
}

pub(crate) const COMBAT_DIE_SIDES: [CombatDieRoll; 12] = [
    CombatDieRoll::new(1, Leader),
    CombatDieRoll::new(1, Leader),
    CombatDieRoll::new(2, Cavalry),
    CombatDieRoll::new(2, Elephant),
    CombatDieRoll::new(3, Elephant),
    CombatDieRoll::new(3, Infantry),
    CombatDieRoll::new(4, Cavalry),
    CombatDieRoll::new(4, Elephant),
    CombatDieRoll::new(5, Cavalry),
    CombatDieRoll::new(5, Infantry),
    CombatDieRoll::new(6, Infantry),
    CombatDieRoll::new(6, Infantry),
];

fn roll(
    game: &mut Game,
    player_index: usize,
    units: &Vec<u32>,
    extra_dies: u8,
    extra_combat_value: i8,
    deny_combat_abilities: bool,
    roll_log: &mut Vec<String>,
) -> CombatRolls {
    let mut dice_rolls = extra_dies;
    let mut unit_types = Units::empty();
    for unit in units {
        let unit = &game.players[player_index].get_unit(*unit).unit_type;
        dice_rolls += 1;
        unit_types += unit;
    }

    let mut rolls = CombatRolls {
        combat_value: extra_combat_value,
        hit_cancels: 0,
    };
    for _ in 0..dice_rolls {
        let dice_roll =
            dice_roll_with_leader_reroll(game, &mut unit_types, deny_combat_abilities, roll_log);
        let value = dice_roll.value;
        rolls.combat_value += value as i8;
        if unit_types.has_unit(&dice_roll.bonus) && !deny_combat_abilities {
            unit_types -= &dice_roll.bonus;

            match dice_roll.bonus {
                Infantry => {
                    rolls.combat_value += 1;
                    add_roll_log_effect(roll_log, "+1 combat value");
                }
                Cavalry => {
                    rolls.combat_value += 2;
                    add_roll_log_effect(roll_log, "+2 combat value");
                }
                Elephant => {
                    rolls.hit_cancels += 1;
                    rolls.combat_value -= value as i8;
                    add_roll_log_effect(roll_log, "-1 hits, no combat value");
                }
                _ => (),
            }
        } else {
            add_roll_log_effect(roll_log, "no bonus");
        }
    }
    if rolls.combat_value < 0 {
        rolls.combat_value = 0;
    }
    rolls
}

fn dice_roll_with_leader_reroll(
    game: &mut Game,
    unit_types: &mut Units,
    deny_combat_abilities: bool,
    roll_log: &mut Vec<String>,
) -> CombatDieRoll {
    let side = roll_die(game, roll_log);

    if deny_combat_abilities || side.bonus != Leader || !unit_types.has_unit(&Leader) {
        return side;
    }

    *unit_types -= &Leader;

    // if used, the leader grants unlimited rerolls of 1s and 2s
    loop {
        add_roll_log_effect(roll_log, "re-roll");
        let side = roll_die(game, roll_log);

        if side.bonus != Leader {
            return side;
        }
    }
}

fn add_roll_log_effect(roll_log: &mut [String], effect: &str) {
    let l = roll_log.len();
    roll_log[l - 1] += &format!("{effect})");
}

fn roll_die(game: &mut Game, roll_log: &mut Vec<String>) -> CombatDieRoll {
    let roll = game.next_dice_roll();
    roll_log.push(format!("{} ({:?}, ", roll.value, roll.bonus));
    roll.clone()
}
