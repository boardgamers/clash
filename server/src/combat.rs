use crate::action::CombatAction;
use crate::game::GameState::Playing;
use crate::game::{Game, GameState};
use crate::map::Terrain::Water;
use crate::position::Position;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader, Ship};
use crate::unit::{UnitType, Units};
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
    pub fn is_compatible_action(&self, action: &CombatAction) -> bool {
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
    #[must_use]
    pub fn new(
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

    #[must_use]
    pub fn active_attackers(&self, game: &Game) -> Vec<u32> {
        active_attackers(game, self.attacker, &self.attackers, self.defender_position)
    }

    #[must_use]
    pub fn defender_fortress(&self, game: &Game) -> bool {
        game.players[self.defender]
            .get_city(self.defender_position)
            .is_some_and(|city| city.pieces.fortress.is_some())
    }
}

enum CombatControl {
    Exit,     // exit to player, doesn't mean the combat has ended
    Continue, // continue to combat loop
}

pub fn initiate_combat(
    game: &mut Game,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attacker_position: Position,
    attackers: Vec<u32>,
    can_retreat: bool,
    next_game_state: Option<GameState>,
) {
    game.lock_undo();
    let initiation = next_game_state.map_or_else(
        || Box::new(mem::replace(&mut game.state, Playing)),
        Box::new,
    );
    combat_loop(
        game,
        Combat::new(
            initiation,
            1,
            CombatPhase::Retreat, // is not used
            defender,
            defender_position,
            attacker,
            attacker_position,
            attackers,
            can_retreat,
        ),
    );
}

//phase is consumed but it is not registered by clippy
///
/// # Panics
///
/// Panics if the action is not compatible with the phase
pub fn execute_combat_action(game: &mut Game, action: CombatAction, mut c: Combat) {
    assert!(c.phase.is_compatible_action(&action), "Illegal action");
    game.lock_undo();
    match action {
        CombatAction::PlayActionCard(card) => {
            assert!(card.is_none());
            //todo use card
            combat_loop(game, c);
            return;
        }
        CombatAction::RemoveCasualties(units) => {
            if matches!(remove_casualties(game, &mut c, units), CombatControl::Exit) {
                return;
            }
        }
        CombatAction::Retreat(action) => {
            if action {
                //todo draw
                return;
            }
            c.round += 1;
        }
    }
    combat_loop(game, c);
}

fn remove_casualties(game: &mut Game, c: &mut Combat, units: Vec<u32>) -> CombatControl {
    let CombatPhase::RemoveCasualties {
        player,
        casualties,
        defender_hits,
    } = c.phase
    else {
        panic!("Illegal action");
    };
    assert_eq!(casualties, units.len() as u8, "Illegal action");
    let (fighting_units, opponent) = if player == c.defender {
        (
            game.players[player]
                .get_units(c.defender_position)
                .iter()
                .map(|unit| unit.id)
                .collect(),
            c.attacker,
        )
    } else if player == c.attacker {
        (c.attackers.clone(), c.defender)
    } else {
        panic!("Illegal action")
    };
    assert!(
        units.iter().all(|unit| fighting_units.contains(unit)),
        "Illegal action"
    );
    for unit in units {
        game.kill_unit(unit, player, opponent);
        if player == c.attacker {
            c.attackers.retain(|id| *id != unit);
        }
    }
    if let Some(defender_hits) = defender_hits {
        if defender_hits < c.attackers.len() as u8 && defender_hits > 0 {
            game.add_info_log_item(format!(
                "\t{} has to remove {} of his attacking units",
                game.players[c.attacker].get_name(),
                defender_hits
            ));
            game.state = GameState::Combat(Combat {
                phase: CombatPhase::RemoveCasualties {
                    player: c.defender,
                    casualties: defender_hits,
                    defender_hits: None,
                },
                ..c.clone()
            });
            return CombatControl::Exit;
        }
        if defender_hits >= c.attackers.len() as u8 {
            for id in mem::take(&mut c.attackers) {
                game.kill_unit(id, c.attacker, c.defender);
            }
        }
    }
    resolve_combat(game, c)
}

fn combat_loop(game: &mut Game, mut c: Combat) {
    loop {
        game.add_info_log_item(format!("\nCombat round {}", c.round));
        //todo: go into tactics phase if either player has tactics card (also if they can not play it unless otherwise specified via setting)

        let active_attackers = c.active_attackers(game);
        let attacker_rolls = roll(game, c.attacker, &active_attackers);
        let active_defenders = active_defenders(game, c.defender, c.defender_position);
        let defender_rolls = roll(game, c.defender, &active_defenders);
        let attacker_combat_value = attacker_rolls.combat_value;
        let attacker_hit_cancels = attacker_rolls.hit_cancels;
        let defender_combat_value = defender_rolls.combat_value;
        let mut defender_hit_cancels = defender_rolls.hit_cancels;
        if c.defender_fortress(game) && c.round == 1 {
            defender_hit_cancels += 1;
        }
        let attacker_hits = (attacker_combat_value / 5).saturating_sub(defender_hit_cancels);
        let defender_hits = (defender_combat_value / 5).saturating_sub(attacker_hit_cancels);
        game.add_info_log_item(format!("\t{} rolled a combined combat value of {attacker_combat_value} and gets {attacker_hits} hits against defending units. {} rolled a combined combat value of {defender_combat_value} and gets {defender_hits} hits against attacking units.", game.players[c.attacker].get_name(), game.players[c.defender].get_name()));
        if attacker_hits < active_defenders.len() as u8 && attacker_hits > 0 {
            kill_some_defenders(game, c, attacker_hits, defender_hits);
            return;
        }
        if attacker_hits >= active_defenders.len() as u8 {
            kill_all_defenders(game, &mut c);
        }
        if defender_hits < active_attackers.len() as u8 && defender_hits > 0 {
            kill_some_attackers(game, c, defender_hits);
            return;
        }
        if defender_hits >= active_attackers.len() as u8 {
            kill_all_attackers(game, &mut c);
        }

        if matches!(resolve_combat(game, &mut c), CombatControl::Exit) {
            return;
        }
    }
}

fn kill_all_attackers(game: &mut Game, c: &mut Combat) {
    for id in &c.attackers {
        game.kill_unit(*id, c.attacker, c.defender);
    }
    c.attackers = vec![];
}

fn kill_some_attackers(game: &mut Game, c: Combat, defender_hits: u8) {
    game.add_info_log_item(format!(
        "\t{} has to remove {} of his attacking units",
        game.players[c.attacker].get_name(),
        defender_hits
    ));
    game.state = GameState::Combat(Combat {
        phase: CombatPhase::RemoveCasualties {
            player: c.attacker,
            casualties: defender_hits,
            defender_hits: None,
        },
        ..c
    });
}

fn kill_all_defenders(game: &mut Game, c: &mut Combat) {
    let defender_units = game.players[c.defender]
        .get_units(c.defender_position)
        .iter()
        .map(|unit| unit.id)
        .collect::<Vec<u32>>();
    for id in defender_units {
        game.kill_unit(id, c.defender, c.attacker);
    }
}

fn kill_some_defenders(game: &mut Game, c: Combat, attacker_hits: u8, defender_hits: u8) {
    game.add_info_log_item(format!(
        "\t{} has to remove {} of his defending units",
        game.players[c.defender].get_name(),
        attacker_hits
    ));
    game.state = GameState::Combat(Combat {
        phase: CombatPhase::RemoveCasualties {
            player: c.defender,
            casualties: attacker_hits,
            defender_hits: Some(defender_hits),
        },
        ..c
    });
}

fn resolve_combat(game: &mut Game, c: &mut Combat) -> CombatControl {
    let active_attackers = c.active_attackers(game);
    let defenders_left = game.players[c.defender].get_units(c.defender_position);
    if active_attackers.is_empty() && defenders_left.is_empty() {
        draw(game, c)
    } else if active_attackers.is_empty() {
        defender_wins(game, c)
    } else if defenders_left.is_empty() {
        attacker_wins(game, c)
    } else if c.can_retreat {
        offer_retreat(game, c)
    } else {
        c.round += 1;
        CombatControl::Continue
    }
}

fn offer_retreat(game: &mut Game, c: &mut Combat) -> CombatControl {
    game.add_info_log_item(format!(
        "\t{} may retreat",
        game.players[c.attacker].get_name()
    ));
    game.state = GameState::Combat(Combat {
        phase: CombatPhase::Retreat,
        ..c.clone()
    });
    CombatControl::Exit
}

fn attacker_wins(game: &mut Game, c: &mut Combat) -> CombatControl {
    game.add_info_log_item(format!(
        "\t{} killed all defending units",
        game.players[c.attacker].get_name()
    ));
    for unit in &c.attackers {
        let unit = game.players[c.attacker]
            .get_unit_mut(*unit)
            .expect("attacker should have all attacking units");
        unit.position = c.defender_position;
    }
    capture_position(game, c.defender, c.defender_position, c.attacker);
    //todo attacker wins
    end_combat(game, c)
}

fn defender_wins(game: &mut Game, c: &mut Combat) -> CombatControl {
    game.add_info_log_item(format!(
        "\t{} killed all attacking units",
        game.players[c.defender].get_name()
    ));
    //todo defender wins
    end_combat(game, c)
}

fn draw(game: &mut Game, c: &mut Combat) -> CombatControl {
    if c.defender_fortress(game) && c.round == 1 {
        game.add_info_log_item(format!("\tAll attacking and defending units where eliminated. {} wins the battle because he has a defending fortress", game.players[c.defender].get_name()));
        //todo defender wins
        return end_combat(game, c);
    }
    game.add_info_log_item(String::from(
        "\tAll attacking and defending units where eliminated, ending the battle in a draw",
    ));
    //todo draw
    end_combat(game, c)
}

pub fn capture_position(game: &mut Game, old_player: usize, position: Position, new_player: usize) {
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

fn end_combat(game: &mut Game, c: &Combat) -> CombatControl {
    game.state = *c.initiation.clone();
    CombatControl::Exit
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

///
/// # Panics
///
/// Panics if the player does not have all units
pub fn roll(game: &mut Game, player_index: usize, units: &Vec<u32>) -> CombatRolls {
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
    let roll = game.get_next_dice_roll();

    if roll > 2 || !unit_types.has_unit(&Leader) {
        return roll;
    }

    *unit_types -= &Leader;

    // if used, the leader grants unlimited rerolls of 1s and 2s
    loop {
        let roll = game.get_next_dice_roll();

        if roll > 2 {
            return roll;
        }
    }
}

#[must_use]
pub fn dice_value(roll: u8) -> u8 {
    roll / 2 + 1
}

/// # Panics
/// if the player does not have the unit
#[must_use]
pub fn active_attackers(
    game: &Game,
    attacker: usize,
    attackers: &[u32],
    defender_position: Position,
) -> Vec<u32> {
    let player = &game.players[attacker];

    let on_water = game.map.tiles[&defender_position] == Water;
    attackers
        .iter()
        .copied()
        .filter(|u| {
            can_fight(
                on_water,
                &player
                    .get_unit(*u)
                    .expect("player should have unit")
                    .unit_type,
            )
        })
        .collect::<Vec<_>>()
}

#[must_use]
pub fn active_defenders(game: &Game, defender: usize, defender_position: Position) -> Vec<u32> {
    let p = &game.players[defender];
    let on_water = game.map.tiles[&defender_position] == Water;
    p.get_units(defender_position)
        .iter()
        .filter(|u| can_fight(on_water, &u.unit_type))
        .map(|u| u.id)
        .collect::<Vec<_>>()
}

#[must_use]
pub fn can_fight(on_water: bool, unit_type: &UnitType) -> bool {
    if on_water {
        matches!(unit_type, Ship)
    } else {
        unit_type.is_army_unit()
    }
}
