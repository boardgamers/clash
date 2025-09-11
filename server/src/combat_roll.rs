use crate::combat::Combat;
use crate::combat_listeners::CombatStrength;
use crate::content::ability::combat_event_origin;
use crate::events::EventPlayer;
use crate::game::Game;
use crate::log::{ActionLogEntry, ActionLogEntryCombatRoll};
use crate::unit::UnitType::{Cavalry, Elephant, Infantry};
use crate::unit::{LEADER_UNIT, UnitType, Units};
use num::Zero;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub(crate) struct CombatHits {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics_card: Option<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub opponent_hit_cancels: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub opponent_fighters: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub combat_value: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub extra_hits: u8, // that cannot be cancelled
}

impl CombatHits {
    #[must_use]
    pub(crate) fn new(
        tactics_card: Option<u8>,
        opponent_hit_cancels: u8,
        opponent_fighters: u8,
        combat_value: u8,
    ) -> CombatHits {
        CombatHits {
            tactics_card,
            opponent_hit_cancels,
            opponent_fighters,
            combat_value,
            extra_hits: 0,
        }
    }

    #[must_use]
    pub(crate) fn hits(&self) -> u8 {
        let hits =
            (self.combat_value / 5).saturating_sub(self.opponent_hit_cancels) + self.extra_hits;
        hits.min(self.opponent_fighters)
    }

    pub(crate) fn all_opponents_killed(&self) -> bool {
        self.hits() == self.opponent_fighters
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UnitCombatRoll {
    pub value: u8,
    pub unit_type: UnitType,
    pub bonus: bool,
}

pub(crate) struct CombatRoundStats {
    player: usize,
    pub(crate) fighters: u8,
    rolls: Vec<UnitCombatRoll>,
    combat_value: u8,
    hit_cancels: u8,
    strength: CombatStrength,
}

impl CombatRoundStats {
    pub(crate) fn roll(
        player: usize,
        c: &Combat,
        game: &mut Game,
        strength: CombatStrength,
    ) -> CombatRoundStats {
        let fighting = c.fighting_units(game, player);
        let mut unit_rolls = vec![];
        let rolls = roll(
            game,
            player,
            &fighting,
            strength.extra_dies,
            strength.extra_combat_value,
            strength.deny_combat_abilities,
            &mut unit_rolls,
        );
        let combat_value = rolls.combat_value as u8;
        let hit_cancels = rolls.hit_cancels + strength.hit_cancels;

        CombatRoundStats {
            strength,
            player,
            rolls: unit_rolls,
            combat_value,
            hit_cancels,
            fighters: fighting.len() as u8,
        }
    }

    pub(crate) fn determine_hits(
        &mut self,
        opponent: &CombatRoundStats,
        game: &mut Game,
        tactics_card: Option<u8>,
    ) -> CombatHits {
        let combat_hits = CombatHits::new(
            tactics_card,
            opponent.hit_cancels,
            opponent.fighters,
            self.combat_value,
        );

        EventPlayer::new(self.player, combat_event_origin()).add_log_entry(
            game,
            ActionLogEntry::CombatRoll(ActionLogEntryCombatRoll {
                rolls: self.rolls.clone(),
                combat_value: self.combat_value,
                hits: combat_hits.hits(),
                combat_modifiers: self.strength.roll_log.clone(),
            }),
        );
        combat_hits
    }
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
    CombatDieRoll::new(1, LEADER_UNIT),
    CombatDieRoll::new(1, LEADER_UNIT),
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
    roll_log: &mut Vec<UnitCombatRoll>,
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
                    add_bonus(roll_log);
                }
                Cavalry => {
                    rolls.combat_value += 2;
                    add_bonus(roll_log);
                }
                Elephant => {
                    rolls.hit_cancels += 1;
                    rolls.combat_value -= value as i8;
                    add_bonus(roll_log);
                }
                _ => (),
            }
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
    roll_log: &mut Vec<UnitCombatRoll>,
) -> CombatDieRoll {
    let side = roll_die(game, roll_log);

    if deny_combat_abilities
        || side.bonus != LEADER_UNIT
        || unit_types.get_amount(&LEADER_UNIT) == 0
    {
        return side;
    }

    *unit_types -= &LEADER_UNIT;

    // if used, the leader grants unlimited rerolls of 1s
    loop {
        add_bonus(roll_log);
        let side = roll_die(game, roll_log);

        if side.bonus != LEADER_UNIT {
            return side;
        }
    }
}

fn add_bonus(roll_log: &mut [UnitCombatRoll]) {
    roll_log.last_mut().expect("entry not found").bonus = true;
}

fn roll_die(game: &mut Game, roll_log: &mut Vec<UnitCombatRoll>) -> CombatDieRoll {
    let roll = game.next_dice_roll();
    roll_log.push(UnitCombatRoll {
        value: roll.value,
        unit_type: roll.bonus,
        bonus: false,
    });
    roll
}
