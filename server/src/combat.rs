use crate::action::{CombatAction, PlayActionCard};
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::game::GameState::Playing;
use crate::game::{Game, GameState};
use crate::position::Position;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader, Ship};
use crate::unit::{UnitType, Units};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Clone, PartialEq)]
pub struct CombatStrength {
    pub attacker: bool,
    pub player_index: usize,
    pub extra_dies: u8,
    pub extra_combat_value: u8,
    pub hit_cancels: u8,
    pub roll_log: Vec<String>,
}

impl CombatStrength {
    #[must_use]
    pub fn new(player_index: usize, attacker: bool) -> Self {
        Self {
            player_index,
            attacker,
            extra_dies: 0,
            extra_combat_value: 0,
            hit_cancels: 0,
            roll_log: vec![],
        }
    }
}

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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum CombatModifier {
    CancelFortressExtraDie,
    CancelFortressIgnoreHit,
    SteelWeaponsAttacker,
    SteelWeaponsDefender,
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<CombatModifier>,
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
            modifiers: vec![],
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
            .is_some_and(|city| city.pieces.fortress.is_some() && self.round == 1)
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
    let initiation = next_game_state.map_or_else(
        || Box::new(mem::replace(&mut game.state, Playing)),
        Box::new,
    );
    let combat = Combat::new(
        initiation,
        1,
        CombatPhase::Retreat, // is not used
        defender,
        defender_position,
        attacker,
        attacker_position,
        attackers,
        can_retreat,
    );

    start_combat(game, combat);
}

pub(crate) fn start_combat(
    game: &mut Game,
    combat: Combat,
) {
    game.lock_undo();
    let attacker = combat.attacker;
    let defender = combat.defender;

    if let Playing = game.state {
        // event listener needs to find the correct state to get the combat position
        game.state = GameState::Combat(combat);
    }
    if game.trigger_custom_phase_event(
        attacker,
        |events| &events.on_combat_start,
        CustomPhaseEventType::StartCombatAttacker,
    )    {
        return;
    }
    if
        game.trigger_custom_phase_event(
            defender,
            |events| &events.on_combat_start,
            CustomPhaseEventType::StartCombatDefender,
        )
    {
        return;
    }

    let GameState::Combat(c) = mem::replace(&mut game.state, GameState::Playing) else {
        panic!("Illegal state");
    };
    combat_loop(game, c);
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
            assert!(matches!(card, PlayActionCard::None));
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
                "\t{} has to remove {} of their attacking units",
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

///
/// # Panics
/// Panics if events are not set
pub fn combat_loop(game: &mut Game, mut c: Combat) {
    loop {
        game.add_info_log_item(format!("\nCombat round {}", c.round));
        //todo: go into tactics phase if either player has tactics card (also if they can not play it unless otherwise specified via setting)

        let attacker_name = game.players[c.attacker].get_name();
        let active_attackers = c.active_attackers(game);
        let mut attacker_strength = CombatStrength::new(c.attacker, true);
        game.players[c.attacker]
            .events
            .as_ref()
            .expect("events should be set")
            .on_combat_round
            .trigger(&mut attacker_strength, &c, game);
        let mut attacker_log = vec![];
        let attacker_rolls = roll(
            game,
            c.attacker,
            &active_attackers,
            attacker_strength.extra_dies,
            attacker_strength.extra_combat_value,
            &mut attacker_log,
        );
        let attacker_log_str = roll_log_str(&attacker_log);

        let active_defenders = active_defenders(game, c.defender, c.defender_position);
        let defender_name = game.players[c.defender].get_name();
        let mut defender_log = vec![];
        let mut defender_strength = CombatStrength::new(c.defender, false);
        game.players[c.defender]
            .events
            .as_ref()
            .expect("events should be set")
            .on_combat_round
            .trigger(&mut defender_strength, &c, game);
        let defender_rolls = roll(
            game,
            c.defender,
            &active_defenders,
            defender_strength.extra_dies,
            defender_strength.extra_combat_value,
            &mut defender_log,
        );
        let defender_log_str = roll_log_str(&defender_log);
        let attacker_combat_value = attacker_rolls.combat_value;
        let attacker_hit_cancels = attacker_rolls.hit_cancels + attacker_strength.hit_cancels;
        let defender_combat_value = defender_rolls.combat_value;
        let defender_hit_cancels = defender_rolls.hit_cancels + defender_strength.hit_cancels;
        let attacker_hits = (attacker_combat_value / 5).saturating_sub(defender_hit_cancels);
        let defender_hits = (defender_combat_value / 5).saturating_sub(attacker_hit_cancels);
        game.add_info_log_item(format!("\t{attacker_name} rolled {attacker_log_str} for combined combat value of {attacker_combat_value} and gets {attacker_hits} hits against defending units. {defender_name} rolled {defender_log_str} for combined combat value of {defender_combat_value} and gets {defender_hits} hits against attacking units."));
        if !attacker_strength.roll_log.is_empty() {
            game.add_info_log_item(format!(
                ". {attacker_name} used the following combat modifiers: {}",
                attacker_strength.roll_log.join(", ")
            ));
        }
        if !defender_strength.roll_log.is_empty() {
            game.add_info_log_item(format!(
                ". {defender_name} used the following combat modifiers: {}",
                defender_strength.roll_log.join(", ")
            ));
        }
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

fn roll_log_str(log: &[String]) -> String {
    if log.is_empty() {
        return String::from("no dice");
    }
    log.join(", ")
}

fn kill_all_attackers(game: &mut Game, c: &mut Combat) {
    for id in &c.attackers {
        game.kill_unit(*id, c.attacker, c.defender);
    }
    c.attackers = vec![];
}

fn kill_some_attackers(game: &mut Game, c: Combat, defender_hits: u8) {
    game.add_info_log_item(format!(
        "\t{} has to remove {} of their attacking units",
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
        .collect_vec();
    for id in defender_units {
        game.kill_unit(id, c.defender, c.attacker);
    }
}

fn kill_some_defenders(game: &mut Game, c: Combat, attacker_hits: u8, defender_hits: u8) {
    game.add_info_log_item(format!(
        "\t{} has to remove {} of their defending units",
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
    let control = end_combat(game, c);
    game.capture_position(c.defender, c.defender_position, c.attacker);
    //todo attacker wins
    control
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
    if c.defender_fortress(game) {
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

fn end_combat(game: &mut Game, c: &Combat) -> CombatControl {
    game.state = *c.initiation.clone();
    CombatControl::Exit
}

struct CombatRolls {
    pub combat_value: u8,
    pub hit_cancels: u8,
}

#[derive(Clone)]
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

///
/// # Panics
///
/// Panics if the player does not have all units
fn roll(
    game: &mut Game,
    player_index: usize,
    units: &Vec<u32>,
    extra_dies: u8,
    extra_combat_value: u8,
    roll_log: &mut Vec<String>,
) -> CombatRolls {
    let mut dice_rolls = extra_dies;
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
        combat_value: extra_combat_value,
        hit_cancels: 0,
    };
    for _ in 0..dice_rolls {
        let dice_roll = dice_roll_with_leader_reroll(game, &mut unit_types, roll_log);
        let value = dice_roll.value;
        rolls.combat_value += value;
        if unit_types.has_unit(&dice_roll.bonus) {
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
                    rolls.combat_value -= value;
                    add_roll_log_effect(roll_log, "-1 hits, no combat value");
                }
                _ => (),
            }
        } else {
            add_roll_log_effect(roll_log, "no bonus");
        }
    }
    rolls
}

fn dice_roll_with_leader_reroll(
    game: &mut Game,
    unit_types: &mut Units,
    roll_log: &mut Vec<String>,
) -> CombatDieRoll {
    let side = roll_die(game, roll_log);

    if side.bonus != Leader || !unit_types.has_unit(&Leader) {
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
    let roll = game.get_next_dice_roll();
    roll_log.push(format!("{} ({:?}, ", roll.value, roll.bonus));
    roll.clone()
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

    let on_water = game.map.is_water(defender_position);
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
        .collect_vec()
}

#[must_use]
pub fn active_defenders(game: &Game, defender: usize, defender_position: Position) -> Vec<u32> {
    let p = &game.players[defender];
    let on_water = game.map.is_water(defender_position);
    p.get_units(defender_position)
        .iter()
        .filter(|u| can_fight(on_water, &u.unit_type))
        .map(|u| u.id)
        .collect_vec()
}

#[must_use]
pub fn can_fight(on_water: bool, unit_type: &UnitType) -> bool {
    if on_water {
        matches!(unit_type, Ship)
    } else {
        unit_type.is_army_unit()
    }
}
