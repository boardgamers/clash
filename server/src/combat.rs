use crate::ability_initializer::AbilityInitializerSetup;
use crate::consts::SHIP_CAPACITY;
use crate::content::builtin::{Builtin, BuiltinBuilder};
use crate::content::custom_phase_actions::{CustomPhaseUnitsRequest, PositionRequest};
use crate::game::GameState::Playing;
use crate::game::{Game, GameState};
use crate::position::Position;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader};
use crate::unit::{UnitType, Units};
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
pub enum CombatResult {
    AttackerWins,
    DefenderWins,
    Draw,
    Retreat,
}

#[derive(Clone, PartialEq)]
pub struct CombatResultInfo {
    pub result: CombatResult,
    pub defender_position: Position,
    pub attacker: usize,
}

impl CombatResultInfo {
    #[must_use]
    pub fn new(result: CombatResult, attacker: usize, defender_position: Position) -> Self {
        Self {
            result,
            defender_position,
            attacker,
        }
    }

    #[must_use]
    pub fn is_defender(&self, player: usize) -> bool {
        self.attacker != player
    }

    #[must_use]
    pub fn is_loser(&self, player: usize) -> bool {
        if self.attacker == player {
            self.result == CombatResult::DefenderWins
        } else {
            self.result == CombatResult::AttackerWins
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Casualties {
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub fighters: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub carried_units: u8,
}

impl Casualties {
    #[must_use]
    pub fn new(fighters: u8, game: &Game, c: &Combat, player: usize) -> Self {
        Self {
            fighters,
            carried_units: c.carried_units_casualties(game, player, fighters),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatRoundResult {
    pub attacker_casualties: Casualties,
    pub defender_casualties: Casualties,
    #[serde(default)]
    pub can_retreat: bool,
    #[serde(default)]
    pub retreated: bool,
}

impl CombatRoundResult {
    #[must_use]
    pub fn new(
        attacker_casualties: Casualties,
        defender_casualties: Casualties,
        can_retreat: bool,
    ) -> Self {
        Self {
            attacker_casualties,
            defender_casualties,
            can_retreat,
            retreated: false,
        }
    }
}

impl CombatRoundResult {
    #[must_use]
    pub fn casualties(&self, attacker: bool) -> &Casualties {
        if attacker {
            &self.attacker_casualties
        } else {
            &self.defender_casualties
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
    pub defender: usize,
    pub defender_position: Position,
    pub attacker: usize,
    pub attacker_position: Position,
    pub attackers: Vec<u32>,
    pub can_retreat: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<CombatModifier>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_result: Option<CombatRoundResult>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CombatResult>,
}

impl Combat {
    #[must_use]
    pub fn new(
        initiation: Box<GameState>,
        round: u32,
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
            defender,
            defender_position,
            attacker,
            attacker_position,
            attackers,
            can_retreat,
            modifiers: vec![],
            round_result: None,
            result: None,
        }
    }

    #[must_use]
    pub fn fighting_units(&self, game: &Game, player: usize) -> Vec<u32> {
        if player == self.attacker {
            self.active_attackers(game)
        } else {
            self.active_defenders(game)
        }
    }

    #[must_use]
    pub(crate) fn active_attackers(&self, game: &Game) -> Vec<u32> {
        let attacker = self.attacker;
        let attackers = &self.attackers;
        let defender_position = self.defender_position;
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
    pub fn active_defenders(&self, game: &Game) -> Vec<u32> {
        let defender = self.defender;
        let defender_position = self.defender_position;
        let p = &game.players[defender];
        let on_water = game.map.is_water(defender_position);
        p.get_units(defender_position)
            .into_iter()
            .filter(|u| can_remove_after_combat(on_water, &u.unit_type))
            .map(|u| u.id)
            .collect_vec()
    }

    #[must_use]
    pub fn defender_fortress(&self, game: &Game) -> bool {
        game.players[self.defender]
            .get_city(self.defender_position)
            .is_some_and(|city| city.pieces.fortress.is_some())
    }

    #[must_use]
    pub fn defender_temple(&self, game: &Game) -> bool {
        game.players[self.defender]
            .get_city(self.defender_position)
            .is_some_and(|city| city.pieces.temple.is_some())
    }

    #[must_use]
    pub fn is_sea_battle(&self, game: &Game) -> bool {
        game.map.is_water(self.defender_position)
    }

    #[must_use]
    pub fn carried_units_casualties(&self, game: &Game, player: usize, casualties: u8) -> u8 {
        if self.is_sea_battle(game) {
            let units = game.players[player].get_units(self.position(player));
            let carried_units = units.iter().filter(|u| u.carrier_id.is_some()).count() as u8;
            let carrier_capacity_left =
                (units.len() as u8 - carried_units - casualties) * SHIP_CAPACITY;
            carried_units.saturating_sub(carrier_capacity_left)
        } else {
            0
        }
    }

    #[must_use]
    pub fn position(&self, player: usize) -> Position {
        if player == self.attacker {
            self.attacker_position
        } else {
            self.defender_position
        }
    }

    #[must_use]
    pub fn enemy(&self, player: usize) -> usize {
        if player == self.attacker {
            self.defender
        } else {
            self.attacker
        }
    }
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
        defender,
        defender_position,
        attacker,
        attacker_position,
        attackers,
        can_retreat,
    );
    game.state = GameState::Combat(combat);
    start_combat(game);
}

pub(crate) fn start_combat(game: &mut Game) {
    game.lock_undo();
    let combat = take_combat(game);
    let attacker = combat.attacker;
    let defender = combat.defender;

    if let Playing = game.state {
        // event listener needs to find the correct state to get the combat position
        game.state = GameState::Combat(combat);
    }

    if game.trigger_custom_phase_event(
        &[attacker, defender],
        |events| &mut events.on_combat_start,
        &(),
        None,
    ) {
        return;
    }

    let c = take_combat(game);
    combat_loop(game, c);
}

pub(crate) fn choose_fighter_casualties() -> Builtin {
    choose_casualties(
        Builtin::builder("Choose Casualties", "Choose which carried units to remove."),
        1,
        |c| c.fighters,
        |game, player| get_combat(game).fighting_units(game, player),
        kill_units,
    )
}

pub(crate) fn choose_carried_units_casualties() -> Builtin {
    choose_casualties(
        Builtin::builder(
            "Choose Casualties (carried units)",
            "Choose which carried units to remove.",
        ),
        2,
        |c| c.carried_units,
        |game, player| {
            let c = get_combat(game);
            let pos = c.position(player);
            let carried: Vec<u32> = game
                .get_player(player)
                .get_units(pos)
                .into_iter()
                .filter(|u| u.carrier_id.is_some())
                .map(|u| u.id)
                .collect();
            carried
        },
        |game, player, units| {
            kill_units(game, player, units);
            save_carried_units(units, game, player, get_combat(game).position(player));
        },
    )
}

pub(crate) fn offer_retreat() -> Builtin {
    Builtin::builder("Offer Retreat", "Do you want to retreat?")
        .add_bool_request(
            |event| &mut event.on_combat_round_end,
            0,
            |game, player, ()| {
                let c = get_combat(game);
                let r = c.round_result.as_ref().expect("no round result");
                if c.attacker == player && r.can_retreat {
                    let p = game.get_player(player);
                    let name = p.get_name();
                    game.add_info_log_item(&format!("{name} can retreat",));
                    true
                } else {
                    false
                }
            },
            |game, _player, player_name, retreat| {
                if retreat {
                    game.add_info_log_item(&format!("{player_name} retreats",));
                } else {
                    game.add_info_log_item(&format!("{player_name} does not retreat",));
                }
                let mut c = take_combat(game);
                c.round_result.as_mut().expect("no round result").retreated = retreat;
                game.state = GameState::Combat(c);
            },
        )
        .build()
}

pub(crate) fn choose_casualties(
    builder: BuiltinBuilder,
    priority: i32,
    get_casualties: impl Fn(&Casualties) -> u8 + 'static + Clone,
    get_choices: impl Fn(&Game, usize) -> Vec<u32> + 'static + Clone,
    kill_units: impl Fn(&mut Game, usize, &[u32]) + 'static + Copy,
) -> Builtin {
    builder
        .add_units_request(
            |event| &mut event.on_combat_round_end,
            priority,
            move |game, player, ()| {
                let c = get_combat(game);
                let r = c.round_result.as_ref().expect("no round result");

                let choices = get_choices(game, player).clone();

                let attacker = player == c.attacker;
                let role = if attacker { "attacking" } else { "defending" };
                let casualties = get_casualties(r.casualties(attacker));
                if casualties == 0 {
                    return None;
                }
                let p = game.get_player(player);
                let name = p.get_name();
                if casualties == choices.len() as u8 {
                    game.add_info_log_item(&format!(
                        "{name} has to remove all of their {role} units",
                    ));
                    kill_units(game, player, &choices);
                    return None;
                }

                let first_type = p
                    .get_unit(*choices.first().expect("no units"))
                    .expect("unit should exist")
                    .unit_type;
                if choices
                    .iter()
                    .all(|u| p.get_unit(*u).expect("unit should exist").unit_type == first_type)
                    || !p.is_human()
                {
                    game.add_info_log_item(&format!(
                        "{name} has to remove {casualties} of their {role} units",
                    ));
                    kill_units(game, player, &choices[..casualties as usize]);
                    return None;
                }

                game.add_info_log_item(&format!(
                    "{name} has to remove {casualties} of their {role} units",
                ));
                Some(CustomPhaseUnitsRequest::new(
                    choices,
                    casualties,
                    Some(format!("Remove {casualties} {role} units")),
                ))
            },
            move |game, player, r| {
                kill_units(game, player, r);
            },
        )
        .build()
}

#[must_use]
pub(crate) fn get_combat(game: &Game) -> &Combat {
    let GameState::Combat(c) = &game.state else {
        panic!("Invalid state")
    };
    c
}

fn kill_units(game: &mut Game, player: usize, killed_unit_ids: &[u32]) {
    let p = game.get_player(player);
    game.add_info_log_item(&format!(
        "{} removed {}",
        p.get_name(),
        killed_unit_ids
            .iter()
            .map(|id| p.get_unit(*id).expect("unit not found").unit_type)
            .collect::<Units>()
    ));

    let mut c = take_combat(game);
    let killer = c.enemy(player);

    for unit in killed_unit_ids {
        game.kill_unit(*unit, player, killer);
        if player == c.attacker {
            c.attackers.retain(|id| id != unit);
        }
    }
    game.state = GameState::Combat(c);
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

pub(crate) fn combat_loop(game: &mut Game, mut c: Combat) {
    loop {
        game.add_info_log_group(format!("Combat round {}", c.round));
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

        let active_defenders = c.active_defenders(game);
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
        let attacker_hits = (attacker_combat_value / 5)
            .saturating_sub(defender_hit_cancels)
            .min(active_defenders.len() as u8);
        let defender_hits = (defender_combat_value / 5)
            .saturating_sub(attacker_hit_cancels)
            .min(active_attackers.len() as u8);
        game.add_info_log_item(&format!("{attacker_name} rolled {attacker_log_str} for combined combat value of {attacker_combat_value} and gets {attacker_hits} hits against defending units."));
        game.add_info_log_item(&format!("{defender_name} rolled {defender_log_str} for combined combat value of {defender_combat_value} and gets {defender_hits} hits against attacking units."));
        if !attacker_strength.roll_log.is_empty() {
            game.add_info_log_item(&format!(
                "{attacker_name} used the following combat modifiers: {}",
                attacker_strength.roll_log.join(", ")
            ));
        }
        if !defender_strength.roll_log.is_empty() {
            game.add_info_log_item(&format!(
                "{defender_name} used the following combat modifiers: {}",
                defender_strength.roll_log.join(", ")
            ));
        }

        let can_retreat = c.can_retreat
            && attacker_hits < active_defenders.len() as u8
            && defender_hits < active_attackers.len() as u8;

        c.round_result = Some(CombatRoundResult::new(
            Casualties::new(defender_hits, game, &c, c.attacker),
            Casualties::new(attacker_hits, game, &c, c.defender),
            can_retreat,
        ));
        game.state = GameState::Combat(c);

        if let Some(r) = combat_round_end(game) {
            c = r;
            c.round_result = None;
        } else {
            return;
        }
    }
}

pub(crate) fn take_combat(game: &mut Game) -> Combat {
    let GameState::Combat(c) = mem::replace(&mut game.state, Playing) else {
        panic!("Illegal state");
    };
    c
}

fn roll_log_str(log: &[String]) -> String {
    if log.is_empty() {
        return String::from("no dice");
    }
    log.join(", ")
}

pub(crate) fn combat_round_end(game: &mut Game) -> Option<Combat> {
    let c = get_combat(game);
    let attacker = c.attacker;
    let defender = c.defender;
    if game.trigger_custom_phase_event(
        &[attacker, defender],
        |events| &mut events.on_combat_round_end,
        &(),
        None,
    ) {
        return None;
    }

    let mut c = take_combat(game);
    let active_attackers = c.active_attackers(game);
    let defenders_left = c.active_defenders(game);
    if active_attackers.is_empty() && defenders_left.is_empty() {
        draw(game, c)
    } else if active_attackers.is_empty() {
        defender_wins(game, c)
    } else if defenders_left.is_empty() {
        attacker_wins(game, c)
    } else if c.round_result.as_ref().expect("no round result").retreated {
        None
    } else {
        c.round += 1;
        Some(c)
    }
}

fn attacker_wins(game: &mut Game, mut c: Combat) -> Option<Combat> {
    game.add_info_log_item("Attacker wins");
    game.move_units(c.attacker, &c.attackers, c.defender_position, None);
    game.capture_position(c.defender, c.defender_position, c.attacker);
    c.result = Some(CombatResult::AttackerWins);
    end_combat(game, c)
}

fn defender_wins(game: &mut Game, mut c: Combat) -> Option<Combat> {
    game.add_info_log_item("Defender wins");
    c.result = Some(CombatResult::DefenderWins);
    end_combat(game, c)
}

pub(crate) fn draw(game: &mut Game, mut c: Combat) -> Option<Combat> {
    if c.defender_fortress(game) && c.round == 1 {
        game.add_info_log_item(&format!(
            "{} wins the battle because he has a defending fortress",
            game.players[c.defender].get_name()
        ));
        // only relevant for event listeners
        c.result = Some(CombatResult::DefenderWins);
        return end_combat(game, c);
    }
    game.add_info_log_item("Battle ends in a draw");
    c.result = Some(CombatResult::Draw);
    end_combat(game, c)
}

pub(crate) fn end_combat(game: &mut Game, combat: Combat) -> Option<Combat> {
    game.lock_undo();
    let attacker = combat.attacker;
    let defender = combat.defender;
    let defender_position = combat.defender_position;
    let info = CombatResultInfo::new(
        combat
            .result
            .clone()
            .expect("Combat result should be set when ending combat"),
        attacker,
        defender_position,
    );

    // set state back to combat for event listeners
    game.state = GameState::Combat(combat);

    if game.trigger_custom_phase_event(
        &[attacker, defender],
        |events| &mut events.on_combat_end,
        &info,
        None,
    ) {
        return None;
    }

    let c = take_combat(game);
    game.state = *c.initiation;
    None
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

#[must_use]
pub fn can_remove_after_combat(on_water: bool, unit_type: &UnitType) -> bool {
    if on_water {
        // carried units may also have to be removed
        true
    } else {
        unit_type.is_army_unit()
    }
}

pub(crate) fn place_settler() -> Builtin {
    Builtin::builder(
        "Place Settler",
        "After losing a city, place a settler in another city.",
    )
    .add_position_request(
        |event| &mut event.on_combat_end,
        0,
        |game, player_index, i| {
            if i.is_defender(player_index)
                && i.is_loser(player_index)
                && game.get_any_city(i.defender_position).is_some()
                && !game.get_player(player_index).cities.is_empty()
                && game.get_player(player_index).available_units().settlers > 0
            {
                let p = game.get_player(player_index);
                let choices: Vec<Position> = p.cities.iter().map(|c| c.position).collect();
                Some(PositionRequest::new(choices, None))
            } else {
                None
            }
        },
        |c, _game, pos| {
            c.add_info_log_item(&format!(
                "{} gained 1 free Settler Unit at {pos} for losing a city",
                c.name,
            ));
            c.gain_unit(c.index, UnitType::Settler, *pos);
        },
    )
    .build()
}
