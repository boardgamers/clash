use crate::action::{CombatAction, PlayActionCard};
use crate::consts::SHIP_CAPACITY;
use crate::content::custom_phase_actions::CustomPhaseEventType;
use crate::game::{Game, GameState};
use crate::position::Position;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader};
use crate::unit::{Unit, UnitType, Units};
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
pub struct RemoveCasualties {
    pub player: usize,
    pub casualties: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub carried_units_casualties: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_hits: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CombatPhase {
    PlayActionCard(usize),
    RemoveCasualties(RemoveCasualties),
    Retreat,
}

impl CombatPhase {
    #[must_use]
    pub fn is_compatible_action(&self, action: &CombatAction) -> bool {
        match self {
            CombatPhase::PlayActionCard(_) => matches!(action, CombatAction::PlayActionCard(_)),
            CombatPhase::RemoveCasualties(_) => {
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

    #[must_use]
    pub fn is_sea_battle(&self, game: &Game) -> bool {
        game.map.is_water(self.defender_position)
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
        || Box::new(mem::replace(&mut game.state, GameState::Playing)),
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

pub(crate) fn start_combat(game: &mut Game, combat: Combat) {
    game.lock_undo();
    let attacker = combat.attacker;
    let defender = combat.defender;

    if let GameState::Playing = game.state {
        // event listener needs to find the correct state to get the combat position
        game.state = GameState::Combat(combat);
    }
    if game.trigger_custom_phase_event(
        attacker,
        |events| &mut events.on_combat_start,
        CustomPhaseEventType::StartCombatAttacker,
        &(),
    ) {
        return;
    }
    if game.trigger_custom_phase_event(
        defender,
        |events| &mut events.on_combat_start,
        CustomPhaseEventType::StartCombatDefender,
        &(),
    ) {
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

fn remove_casualties(game: &mut Game, c: &mut Combat, killed_unit_ids: Vec<u32>) -> CombatControl {
    let CombatPhase::RemoveCasualties(r) = &c.phase else {
        panic!("Illegal action");
    };
    let player = r.player;
    let (fighting_units, killer, pos) = if player == c.defender {
        (
            game.players[player]
                .get_units(c.defender_position)
                .iter()
                .map(|unit| unit.id)
                .collect(),
            c.attacker,
            c.defender_position,
        )
    } else if player == c.attacker {
        (c.attackers.clone(), c.defender, c.attacker_position)
    } else {
        panic!("Illegal action")
    };
    let units: Vec<&Unit> = killed_unit_ids
        .iter()
        .map(|id| game.players[player].get_unit(*id).expect("unit not found"))
        .collect();
    let ships: Vec<&Unit> = units
        .clone()
        .into_iter()
        .filter(|u| u.unit_type.is_ship())
        .collect();
    let land_units = units.len() as u8 - ships.len() as u8;
    let casualties = r.casualties;
    if game.map.is_water(c.defender_position) {
        assert!(
            ships.iter().all(|unit| fighting_units.contains(&unit.id)),
            "Illegal action"
        );
        assert_eq!(
            r.carried_units_casualties, land_units,
            "Illegal carried units"
        );
        assert_eq!(casualties, ships.len() as u8, "Illegal action");

        save_carried_units(&killed_unit_ids, game, player, pos);
    } else {
        assert!(
            killed_unit_ids
                .iter()
                .all(|unit| fighting_units.contains(unit)),
            "Illegal action"
        );
        assert_eq!(land_units, killed_unit_ids.len() as u8, "Illegal units");
        assert_eq!(casualties, land_units, "Illegal action");
    }

    for unit in killed_unit_ids {
        game.kill_unit(unit, player, killer);
        if player == c.attacker {
            c.attackers.retain(|id| *id != unit);
        }
    }

    if let Some(defender_hits) = r.defender_hits {
        if defender_hits > 0 {
            if defender_hits < c.attackers.len() as u8 {
                game.add_info_log_item(format!(
                    "\t{} has to remove {} of their attacking units",
                    game.players[c.attacker].get_name(),
                    defender_hits
                ));
                to_remove_casualties(game, c.defender, c.clone(), defender_hits, None);
                return CombatControl::Exit;
            }
            kill_all_attackers(game, c);
        }
    }
    resolve_combat(game, c)
}

fn save_carried_units(killed_unit_ids: &[u32], game: &mut Game, player: usize, pos: Position) {
    let mut carried_units: HashMap<u32, u8> = HashMap::new();

    for unit in game.get_player(player).clone().get_units(pos) {
        if killed_unit_ids.contains(&unit.id) {
            continue;
        }
        if let Some(carrier) = unit.carrier_id {
            carried_units
                .entry(carrier)
                .and_modify(|e| *e += 1)
                .or_insert(1);
        } else {
            carried_units.entry(unit.id).or_insert(0);
        }
    }

    // embark to surviving ships
    for unit in game.get_player(player).clone().get_units(pos) {
        let unit = game.players[player]
            .get_unit_mut(unit.id)
            .expect("unit not found");
        if unit
            .carrier_id
            .is_some_and(|id| killed_unit_ids.contains(&id))
        {
            let (&carrier_id, &carried) = carried_units
                .iter()
                .find(|(_carrier_id, carried)| **carried < SHIP_CAPACITY)
                .expect("no carrier found to save carried units");
            carried_units.insert(carrier_id, carried + 1);
            unit.carrier_id = Some(carrier_id);
        }
    }
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
        let _ = game.players[c.attacker]
            .events
            .on_combat_round
            .get()
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
        let _ = game.players[c.defender]
            .events
            .on_combat_round
            .get()
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
            kill_some_units(
                game,
                c.defender,
                c,
                attacker_hits,
                "defending",
                Some(defender_hits),
            );
            return;
        }
        if attacker_hits >= active_defenders.len() as u8 {
            kill_all_defenders(game, &mut c);
        }
        if defender_hits < active_attackers.len() as u8 && defender_hits > 0 {
            kill_some_units(game, c.attacker, c, defender_hits, "attacking", None);
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

fn kill_some_units(
    game: &mut Game,
    player: usize,
    c: Combat,
    casualties: u8,
    role: &str,
    defender_hits: Option<u8>,
) {
    game.add_info_log_item(format!(
        "\t{} has to remove {casualties} of their {role} units",
        game.players[player].get_name(),
    ));
    to_remove_casualties(game, player, c, casualties, defender_hits);
}

fn to_remove_casualties(
    game: &mut Game,
    player: usize,
    c: Combat,
    casualties: u8,
    defender_hits: Option<u8>,
) {
    let carried_units_casualties = if c.is_sea_battle(game) {
        let home_position = if player == c.attacker {
            c.attacker_position
        } else {
            c.defender_position
        };
        let units = game.players[player].get_units(home_position);
        let carried_units = units.iter().filter(|u| u.carrier_id.is_some()).count() as u8;
        let carrier_capacity_left =
            (units.len() as u8 - carried_units - casualties) * SHIP_CAPACITY;
        carried_units.saturating_sub(carrier_capacity_left)
    } else {
        0
    };

    game.state = GameState::Combat(Combat {
        phase: CombatPhase::RemoveCasualties(RemoveCasualties {
            player,
            casualties,
            carried_units_casualties,
            defender_hits,
        }),
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
    game.move_units(c.attacker, &c.attackers, c.defender_position, None);
    let control = end_combat(game, c);
    game.capture_position(c.defender, c.defender_position, c.attacker);
    control
}

fn defender_wins(game: &mut Game, c: &mut Combat) -> CombatControl {
    game.add_info_log_item(format!(
        "\t{} killed all attacking units",
        game.players[c.defender].get_name()
    ));
    end_combat(game, c)
}

fn draw(game: &mut Game, c: &mut Combat) -> CombatControl {
    if c.defender_fortress(game) {
        game.add_info_log_item(format!("\tAll attacking and defending units where eliminated. {} wins the battle because he has a defending fortress", game.players[c.defender].get_name()));
        return end_combat(game, c);
    }
    game.add_info_log_item(String::from(
        "\tAll attacking and defending units where eliminated, ending the battle in a draw",
    ));
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
            can_remove_after_combat(
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
        .filter(|u| can_remove_after_combat(on_water, &u.unit_type))
        .map(|u| u.id)
        .collect_vec()
}

#[must_use]
pub fn can_remove_after_combat(on_water: bool, unit_type: &UnitType) -> bool {
    if on_water {
        // carried units may also have to be removed
        true
    } else {
        unit_type.is_army_unit()
    }
}
