use crate::action::CombatAction;
use crate::game::GameState::Playing;
use crate::game::{Game, GameState};
use crate::map::Terrain::Water;
use crate::position::Position;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader, Ship};
use crate::unit::Units;
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CombatPhase {
    PlayActionCard(usize),
    RemoveCasualties {
        player: usize,
        casualties: u8,
        defender_hits: Option<u8>,
    },
    Retreat,
}

impl CombatPhase {
    #[must_use]
    pub(crate) fn is_compatible_action(&self, action: &CombatAction) -> bool {
        match self {
            CombatPhase::PlayActionCard(_) => matches!(action, CombatAction::PlayActionCard(_)),
            CombatPhase::RemoveCasualties { .. } => {
                matches!(action, CombatAction::RemoveCasualties(_))
            }
            CombatPhase::Retreat => matches!(action, CombatAction::Retreat(_)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Combat {
    pub initiation: Box<GameState>,
    pub round: u32, //starts with one,
    pub phase: CombatPhase,
    pub defender: usize,
    pub defender_position: Position,
    pub attacker: usize,
    pub attacker_position: Position,
    pub attackers: Vec<u32>,
    pub can_retreat: bool,
}

impl Combat {
    fn new(
        initiation: Box<GameState>,
        round: u32,
        phase: CombatPhase,
        defender: usize,
        defender_position: Position,
        attacker: usize,
        attacker_position: Position,
        attackers: Vec<u32>,
        can_retreat: bool,
    ) -> Self {
        Self {
            initiation,
            round,
            phase,
            defender,
            defender_position,
            attacker,
            attacker_position,
            attackers,
            can_retreat,
        }
    }
}

pub(crate) fn initiate_combat(
    game: &mut Game,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attacker_position: Position,
    mut attackers: Vec<u32>,
    can_retreat: bool,
    next_game_state: Option<GameState>,
) {
    let mut round = 1;
    game.lock_undo();
    combat_loop(
        game,
        next_game_state.map(Box::new),
        &mut round,
        defender,
        defender_position,
        attacker,
        attacker_position,
        &mut attackers,
        can_retreat,
    );
}

#[allow(clippy::needless_pass_by_value)]
//phase is consumed but it is not registered by clippy
pub(crate) fn execute_combat_action(
    game: &mut Game,
    action: CombatAction,
    initiation: Box<GameState>,
    mut round: u32,
    phase: CombatPhase,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attacker_position: Position,
    mut attackers: Vec<u32>,
    can_retreat: bool,
) {
    assert!(phase.is_compatible_action(&action), "Illegal action");
    game.lock_undo();
    match action {
        CombatAction::PlayActionCard(card) => {
            assert!(card.is_none());
            //todo use card
            combat_loop(
                game,
                Some(initiation),
                &mut round,
                defender,
                defender_position,
                attacker,
                attacker_position,
                &mut attackers,
                can_retreat,
            );
            return;
        }
        CombatAction::RemoveCasualties(units) => {
            let CombatPhase::RemoveCasualties {
                player,
                casualties,
                defender_hits,
            } = phase
            else {
                panic!("Illegal action");
            };
            assert_eq!(casualties, units.len() as u8, "Illegal action");
            let (fighting_units, opponent) = if player == defender {
                (
                    game.players[player]
                        .get_units(defender_position)
                        .iter()
                        .map(|unit| unit.id)
                        .collect(),
                    attacker,
                )
            } else if player == attacker {
                (attackers.clone(), defender)
            } else {
                panic!("Illegal action")
            };
            assert!(
                units.iter().all(|unit| fighting_units.contains(unit)),
                "Illegal action"
            );
            for unit in units {
                game.kill_unit(unit, player, opponent);
                if player == attacker {
                    attackers.retain(|id| *id != unit);
                }
            }
            if let Some(defender_hits) = defender_hits {
                if defender_hits < attackers.len() as u8 && defender_hits > 0 {
                    game.add_info_log_item(format!(
                        "\t{} has to remove {} of his attacking units",
                        game.players[attacker].get_name(),
                        defender_hits
                    ));
                    game.state = GameState::Combat(Combat::new(
                        initiation,
                        round,
                        CombatPhase::RemoveCasualties {
                            player: defender,
                            casualties: defender_hits,
                            defender_hits: None,
                        },
                        defender,
                        defender_position,
                        attacker,
                        attacker_position,
                        attackers,
                        can_retreat,
                    ));
                    return;
                }
                if defender_hits >= attackers.len() as u8 {
                    for id in mem::take(&mut attackers) {
                        game.kill_unit(id, attacker, defender);
                    }
                }
            }
            let defenders_left = game.players[defender].get_units(defender_position).len();
            if attackers.is_empty() && defenders_left == 0 {
                //todo if the defender has a fortress he wins
                game.add_info_log_item(String::from("\tAll attacking and defending units killed each other, ending the battle in a draw"));
                //todo otherwise: draw
                game.state = *initiation;
                return;
            }
            if attackers.is_empty() {
                game.add_info_log_item(format!(
                    "\t{} killed all attacking units and wins",
                    game.players[defender].get_name()
                ));
                //todo defender wins
                game.state = *initiation;
                return;
            }
            if defenders_left == 0 {
                game.add_info_log_item(format!(
                    "\t{} killed all defending units and wins",
                    game.players[attacker].get_name()
                ));
                for unit in &attackers {
                    let unit = game.players[attacker]
                        .get_unit_mut(*unit)
                        .expect("attacker should have all attacking units");
                    unit.position = defender_position;
                }
                capture_position(game, defender, defender_position, attacker);
                //todo attacker wins
                game.state = *initiation;
                return;
            }
            if can_retreat {
                game.add_info_log_item(format!(
                    "\t{} may retreat",
                    game.players[attacker].get_name()
                ));
                game.state = GameState::Combat(Combat::new(
                    initiation,
                    round,
                    CombatPhase::Retreat,
                    defender,
                    defender_position,
                    attacker,
                    attacker_position,
                    attackers,
                    true,
                ));
                return;
            }
            round += 1;
        }
        CombatAction::Retreat(action) => {
            if action {
                //todo draw
                return;
            }
            round += 1;
        }
    }
    combat_loop(
        game,
        Some(initiation),
        &mut round,
        defender,
        defender_position,
        attacker,
        attacker_position,
        &mut attackers,
        can_retreat,
    );
}

fn combat_loop(
    game: &mut Game,
    initiation: Option<Box<GameState>>,
    round: &mut u32,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attacker_position: Position,
    attackers: &mut Vec<u32>,
    can_retreat: bool,
) {
    let defender_fortress = game.players[defender]
        .get_city(defender_position)
        .is_some_and(|city| city.pieces.fortress.is_some());
    loop {
        game.add_info_log_item(format!("\nCombat round {round}"));
        //todo: go into tactics phase if either player has tactics card (also if they can not play it unless otherwise specified via setting)

        let attacker_rolls = roll(game, attacker, attackers);
        let defender_units = defenders(game, defender, defender_position);
        let defender_rolls = roll(game, defender, &defender_units);
        let attacker_combat_value = attacker_rolls.combat_value;
        let attacker_hit_cancels = attacker_rolls.hit_cancels;
        let defender_combat_value = defender_rolls.combat_value;
        let mut defender_hit_cancels = defender_rolls.hit_cancels;
        if defender_fortress && *round == 1 {
            defender_hit_cancels += 1;
        }
        let attacker_hits = (attacker_combat_value / 5).saturating_sub(defender_hit_cancels);
        let defender_hits = (defender_combat_value / 5).saturating_sub(attacker_hit_cancels);
        game.add_info_log_item(format!("\t{} rolled a combined combat value of {attacker_combat_value} and gets {attacker_hits} hits against defending units. {} rolled a combined combat value of {defender_combat_value} and gets {defender_hits} hits against attacking units.", game.players[attacker].get_name(), game.players[defender].get_name()));
        if attacker_hits < defender_units.len() as u8 && attacker_hits > 0 {
            game.add_info_log_item(format!(
                "\t{} has to remove {} of his defending units",
                game.players[defender].get_name(),
                attacker_hits
            ));
            game.state = GameState::Combat(Combat::new(
                get_initiation(game, initiation),
                *round,
                CombatPhase::RemoveCasualties {
                    player: defender,
                    casualties: attacker_hits,
                    defender_hits: Some(defender_hits),
                },
                defender,
                defender_position,
                attacker,
                attacker_position,
                attackers.clone(),
                can_retreat,
            ));
            return;
        }
        if attacker_hits >= defender_units.len() as u8 {
            let defender_units = game.players[defender]
                .get_units(defender_position)
                .iter()
                .map(|unit| unit.id)
                .collect::<Vec<u32>>();
            for id in defender_units {
                game.kill_unit(id, defender, attacker);
            }
        }
        if defender_hits < attackers.len() as u8 && defender_hits > 0 {
            game.add_info_log_item(format!(
                "\t{} has to remove {} of his attacking units",
                game.players[attacker].get_name(),
                defender_hits
            ));
            game.state = GameState::Combat(Combat::new(
                get_initiation(game, initiation),
                *round,
                CombatPhase::RemoveCasualties {
                    player: attacker,
                    casualties: defender_hits,
                    defender_hits: None,
                },
                defender,
                defender_position,
                attacker,
                attacker_position,
                attackers.clone(),
                can_retreat,
            ));
            return;
        }
        if defender_hits >= attackers.len() as u8 {
            for id in mem::take(attackers) {
                game.kill_unit(id, attacker, defender);
            }
        }
        let defenders_left = game.players[defender].get_units(defender_position);
        if attackers.is_empty() && defenders_left.is_empty() {
            if defender_fortress && *round == 1 {
                game.add_info_log_item(format!("\tAll attacking and defending units where eliminated. {} wins the battle because he has a defending fortress", game.players[defender].get_name()));
                //todo defender wins
                end_combat(game, initiation);
                return;
            }
            game.add_info_log_item(String::from(
                "\tAll attacking and defending units where eliminated, ending the battle in a draw",
            ));
            //todo draw
            end_combat(game, initiation);
            return;
        }
        if attackers.is_empty() {
            game.add_info_log_item(format!(
                "\t{} killed all attacking units",
                game.players[defender].get_name()
            ));
            //todo defender wins
            end_combat(game, initiation);
            return;
        }
        if defenders_left.is_empty() {
            game.add_info_log_item(format!(
                "\t{} killed all defending units",
                game.players[attacker].get_name()
            ));
            for unit in &*attackers {
                let unit = game.players[attacker]
                    .get_unit_mut(*unit)
                    .expect("attacker should have all attacking units");
                unit.position = defender_position;
            }
            end_combat(game, initiation);
            capture_position(game, defender, defender_position, attacker);
            //todo attacker wins
            return;
        }
        if can_retreat {
            game.add_info_log_item(format!(
                "\t{} may retreat",
                game.players[attacker].get_name()
            ));
            game.state = GameState::Combat(Combat::new(
                get_initiation(game, initiation),
                *round,
                CombatPhase::Retreat,
                defender,
                defender_position,
                attacker,
                attacker_position,
                attackers.clone(),
                true,
            ));
            return;
        }
        *round += 1;
    }
}

pub(crate) fn capture_position(
    game: &mut Game,
    old_player: usize,
    position: Position,
    new_player: usize,
) {
    let captured_settlers = game.players[old_player]
        .get_units(position)
        .iter()
        .map(|unit| unit.id)
        .collect::<Vec<u32>>();
    if !captured_settlers.is_empty() {
        game.add_to_last_log_item(&format!(
            " and captured {} settlers of {}",
            captured_settlers.len(),
            game.players[old_player].get_name()
        ));
    }
    for id in captured_settlers {
        game.players[old_player].remove_unit(id);
    }
    game.conquer_city(position, new_player, old_player);
}

fn end_combat(game: &mut Game, initiation: Option<Box<GameState>>) {
    game.state = *get_initiation(game, initiation);
}

#[allow(clippy::unnecessary_box_returns)]
fn get_initiation(game: &mut Game, initiation: Option<Box<GameState>>) -> Box<GameState> {
    initiation.unwrap_or_else(|| Box::new(mem::replace(&mut game.state, Playing)))
}

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
    // if used, the leader grants unlimited rerolls of 1s and 2s
    let can_reroll = unit_types.has_unit(&Leader);
    let mut leader_used = false;
    loop {
        let roll = game.get_next_dice_roll();

        if roll > 2 || !can_reroll {
            return roll;
        }
        if !leader_used {
            leader_used = true;
            *unit_types -= &Leader;
        }
    }
}

#[must_use]
pub(crate) fn dice_value(roll: u8) -> u8 {
    roll / 2 + 1
}

#[must_use]
pub fn defenders(game: &Game, defender: usize, defender_position: Position) -> Vec<u32> {
    let p = &game.players[defender];
    let defenders = if game.map.tiles[&defender_position] == Water {
        p.get_units(defender_position)
            .iter()
            .filter(|u| u.unit_type == Ship)
            .map(|u| u.id)
            .collect::<Vec<_>>()
    } else {
        p.get_units(defender_position)
            .iter()
            .filter(|u| u.unit_type.is_army_unit())
            .map(|u| u.id)
            .collect::<Vec<_>>()
    };
    defenders
}
