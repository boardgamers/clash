use crate::city::MoodState::Angry;
use crate::city_pieces::Building;
use crate::combat_listeners::{
    Casualties, CombatResult, CombatResultInfo, CombatRoundResult, CombatStrength,
};
use crate::consts::SHIP_CAPACITY;
use crate::content::custom_phase_actions::CurrentEventType;
use crate::game::{Game, GameState};
use crate::movement::{move_units, stop_current_move};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType::{Cavalry, Elephant, Infantry, Leader};
use crate::unit::{MoveUnits, MovementRestriction, UnitType, Units};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum CombatModifier {
    CancelFortressExtraDie,
    CancelFortressIgnoreHit,
    SteelWeaponsAttacker,
    SteelWeaponsDefender,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum CombatRetreatState {
    CanRetreat,
    CannotRetreat,
    Retreated,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Combat {
    pub round: u32, //starts with one,
    pub defender: usize,
    pub defender_position: Position,
    pub attacker: usize,
    pub attacker_position: Position,
    pub attackers: Vec<u32>,
    pub retreat: CombatRetreatState,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<CombatModifier>,
}

impl Combat {
    #[must_use]
    pub fn new(
        round: u32,
        defender: usize,
        defender_position: Position,
        attacker: usize,
        attacker_position: Position,
        attackers: Vec<u32>,
        can_retreat: bool,
    ) -> Self {
        Self {
            round,
            defender,
            defender_position,
            attacker,
            attacker_position,
            attackers,
            modifiers: vec![],
            retreat: if can_retreat {
                CombatRetreatState::CanRetreat
            } else {
                CombatRetreatState::CannotRetreat
            },
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

        let on_water = game.map.is_sea(defender_position);
        attackers
            .iter()
            .copied()
            .filter(|u| can_remove_after_combat(on_water, &player.get_unit(*u).unit_type))
            .collect_vec()
    }

    #[must_use]
    pub fn active_defenders(&self, game: &Game) -> Vec<u32> {
        let defender = self.defender;
        let defender_position = self.defender_position;
        let p = &game.players[defender];
        let on_water = game.map.is_sea(defender_position);
        p.get_units(defender_position)
            .into_iter()
            .filter(|u| can_remove_after_combat(on_water, &u.unit_type))
            .map(|u| u.id)
            .collect_vec()
    }

    #[must_use]
    pub fn defender_fortress(&self, game: &Game) -> bool {
        game.players[self.defender]
            .try_get_city(self.defender_position)
            .is_some_and(|city| city.pieces.fortress.is_some())
    }

    #[must_use]
    pub fn defender_temple(&self, game: &Game) -> bool {
        game.players[self.defender]
            .try_get_city(self.defender_position)
            .is_some_and(|city| city.pieces.temple.is_some())
    }

    #[must_use]
    pub fn is_sea_battle(&self, game: &Game) -> bool {
        game.map.is_sea(self.defender_position)
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
    pub fn opponent(&self, player: usize) -> usize {
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
) {
    let combat = Combat::new(
        1,
        defender,
        defender_position,
        attacker,
        attacker_position,
        attackers,
        can_retreat,
    );
    game.push_state(GameState::Combat(combat));
    start_combat(game);
}

pub(crate) fn start_combat(game: &mut Game) {
    game.lock_undo();
    let combat = get_combat(game);
    let attacker = combat.attacker;
    let defender = combat.defender;

    if game.trigger_current_event(
        &[attacker, defender],
        |events| &mut events.on_combat_start,
        &(),
        |()| CurrentEventType::CombatStart,
        None,
    ) {
        return;
    }

    let c = take_combat(game);
    combat_loop(game, c);
}

#[must_use]
pub(crate) fn get_combat(game: &Game) -> &Combat {
    let GameState::Combat(c) = &game.state() else {
        panic!("Invalid state")
    };
    c
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

        let can_retreat = matches!(c.retreat, CombatRetreatState::CanRetreat)
            && attacker_hits < active_defenders.len() as u8
            && defender_hits < active_attackers.len() as u8;

        let result = CombatRoundResult::new(
            Casualties::new(defender_hits, game, &c, c.attacker),
            Casualties::new(attacker_hits, game, &c, c.defender),
            can_retreat,
        );
        game.push_state(GameState::Combat(c));

        if let Some(r) = combat_round_end(game, &result) {
            c = r;
        } else {
            return;
        }
    }
}

pub(crate) fn take_combat(game: &mut Game) -> Combat {
    let GameState::Combat(c) = game.pop_state() else {
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

pub(crate) fn combat_round_end(game: &mut Game, r: &CombatRoundResult) -> Option<Combat> {
    let c = get_combat(game);
    let attacker = c.attacker;
    let defender = c.defender;
    if game.trigger_current_event(
        &[attacker, defender],
        |events| &mut events.on_combat_round_end,
        r,
        CurrentEventType::CombatRoundEnd,
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
    } else if matches!(c.retreat, CombatRetreatState::Retreated) {
        None
    } else {
        c.round += 1;
        Some(c)
    }
}

fn attacker_wins(game: &mut Game, c: Combat) -> Option<Combat> {
    game.add_info_log_item("Attacker wins");
    move_units(game, c.attacker, &c.attackers, c.defender_position, None);
    capture_position(game, c.defender, c.defender_position, c.attacker);
    end_combat(game, c, CombatResult::AttackerWins)
}

fn defender_wins(game: &mut Game, c: Combat) -> Option<Combat> {
    game.add_info_log_item("Defender wins");
    end_combat(game, c, CombatResult::DefenderWins)
}

pub(crate) fn draw(game: &mut Game, c: Combat) -> Option<Combat> {
    if c.defender_fortress(game) && c.round == 1 {
        game.add_info_log_item(&format!(
            "{} wins the battle because he has a defending fortress",
            game.players[c.defender].get_name()
        ));
        return end_combat(game, c, CombatResult::DefenderWins);
    }
    game.add_info_log_item("Battle ends in a draw");
    end_combat(game, c, CombatResult::Draw)
}

pub(crate) fn end_combat(game: &mut Game, combat: Combat, r: CombatResult) -> Option<Combat> {
    game.lock_undo();
    let attacker = combat.attacker;
    let defender = combat.defender;
    let defender_position = combat.defender_position;
    let info = CombatResultInfo::new(r, attacker, defender, defender_position);

    // set state back to combat for event listeners
    game.push_state(GameState::Combat(combat));

    if game.trigger_current_event(
        &[attacker, defender],
        |events| &mut events.on_combat_end,
        &info,
        |i| CurrentEventType::CombatEnd(i.result),
        None,
    ) {
        return None;
    }

    take_combat(game);
    if let Some(GameState::Movement(_)) = game.state_stack.last() {
        stop_current_move(game);
    }
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
        let unit = &game.players[player_index].get_unit(*unit).unit_type;
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

pub(crate) fn conquer_city(
    game: &mut Game,
    position: Position,
    new_player_index: usize,
    old_player_index: usize,
) {
    let Some(mut city) = game.players[old_player_index].take_city(position) else {
        panic!("player should have this city")
    };
    // undo would only be possible if the old owner can't spawn a settler
    // and this would be hard to understand
    game.lock_undo();
    game.add_to_last_log_item(&format!(
        " and captured {}'s city at {position}",
        game.players[old_player_index].get_name()
    ));
    let attacker_is_human = game.get_player(new_player_index).is_human();
    let size = city.mood_modified_size(&game.players[new_player_index]);
    if attacker_is_human {
        game.players[new_player_index].gain_resources(ResourcePile::gold(size as u32));
    }
    let take_over = game.get_player(new_player_index).is_city_available();

    if take_over {
        city.player_index = new_player_index;
        city.mood_state = Angry;
        if attacker_is_human {
            for wonder in &city.pieces.wonders {
                (wonder.listeners.deinitializer)(game, old_player_index);
                (wonder.listeners.initializer)(game, new_player_index);
            }

            for (building, owner) in city.pieces.building_owners() {
                if matches!(building, Building::Obelisk) {
                    continue;
                }
                let Some(owner) = owner else {
                    continue;
                };
                if owner != old_player_index {
                    continue;
                }
                if game.players[new_player_index].is_building_available(building, game) {
                    city.pieces.set_building(building, new_player_index);
                } else {
                    city.pieces.remove_building(building);
                    game.players[new_player_index].gain_resources(ResourcePile::gold(1));
                }
            }
        }
        game.players[new_player_index].cities.push(city);
    } else {
        game.players[new_player_index].gain_resources(ResourcePile::gold(city.size() as u32));
        city.raze(game, old_player_index);
    }
}

pub fn capture_position(game: &mut Game, old_player: usize, position: Position, new_player: usize) {
    let captured_settlers = game.players[old_player]
        .get_units(position)
        .iter()
        .map(|unit| unit.id)
        .collect_vec();
    if !captured_settlers.is_empty() {
        game.add_to_last_log_item(&format!(
            " and killed {} settlers of {}",
            captured_settlers.len(),
            game.players[old_player].get_name()
        ));
    }
    for id in captured_settlers {
        game.players[old_player].remove_unit(id);
    }
    if game.get_player(old_player).try_get_city(position).is_some() {
        conquer_city(game, position, new_player, old_player);
    }
}

fn move_to_defended_tile(
    game: &mut Game,
    player_index: usize,
    units: &Vec<u32>,
    destination: Position,
    starting_position: Position,
    defender: usize,
) -> bool {
    let has_defending_units = game.players[defender]
        .get_units(destination)
        .iter()
        .any(|unit| !unit.unit_type.is_settler());
    let has_fortress = game.players[defender]
        .try_get_city(destination)
        .is_some_and(|city| city.pieces.fortress.is_some());

    let mut military = false;
    for unit_id in units {
        let unit = game.players[player_index].get_unit_mut(*unit_id);
        if !unit.unit_type.is_settler() {
            if unit
                .movement_restrictions
                .contains(&MovementRestriction::Battle)
            {
                panic!("unit can't attack");
            }
            unit.movement_restrictions.push(MovementRestriction::Battle);
            military = true;
        }
    }
    assert!(military, "Need military units to attack");

    if has_defending_units || has_fortress {
        initiate_combat(
            game,
            defender,
            destination,
            player_index,
            starting_position,
            units.clone(),
            game.get_player(player_index).is_human(),
        );
        return true;
    }
    false
}

pub(crate) fn move_with_possible_combat(
    game: &mut Game,
    player_index: usize,
    starting_position: Position,
    m: &MoveUnits,
) -> bool {
    let enemy = game.enemy_player(player_index, m.destination);
    if let Some(defender) = enemy {
        if move_to_defended_tile(
            game,
            player_index,
            &m.units,
            m.destination,
            starting_position,
            defender,
        ) {
            return true;
        }
    } else {
        move_units(
            game,
            player_index,
            &m.units,
            m.destination,
            m.embark_carrier_id,
        );
    }

    if let Some(enemy) = enemy {
        capture_position(game, enemy, m.destination, player_index);
    }
    false
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use super::{conquer_city, Game};

    use crate::game::GameState;
    use crate::payment::PaymentOptions;
    use crate::utils::tests::FloatEq;
    use crate::{
        city::{City, MoodState::*},
        city_pieces::Building::*,
        content::civilizations,
        map::Map,
        player::Player,
        position::Position,
        utils::Rng,
        wonder::Wonder,
    };

    #[must_use]
    pub fn test_game() -> Game {
        Game {
            state_stack: vec![GameState::Playing],
            current_events: Vec::new(),
            players: Vec::new(),
            map: Map::new(HashMap::new()),
            starting_player_index: 0,
            current_player_index: 0,
            action_log: Vec::new(),
            action_log_index: 0,
            log: [
                String::from("The game has started"),
                String::from("Age 1 has started"),
                String::from("Round 1/3"),
            ]
            .iter()
            .map(|s| vec![s.to_string()])
            .collect(),
            undo_limit: 0,
            actions_left: 3,
            successful_cultural_influence: false,
            round: 1,
            age: 1,
            messages: vec![String::from("Game has started")],
            rng: Rng::from_seed(1_234_567_890),
            dice_roll_outcomes: Vec::new(),
            dice_roll_log: Vec::new(),
            dropped_players: Vec::new(),
            wonders_left: Vec::new(),
            wonder_amount_left: 0,
            incidents_left: Vec::new(),
            permanent_incident_effects: Vec::new(),
            undo_context_stack: Vec::new(),
        }
    }

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", "test", PaymentOptions::free(), vec![]).build();
        let mut game = test_game();
        game.players.push(old);
        game.players.push(new);
        let old = 0;
        let new = 1;

        let position = Position::new(0, 0);
        game.players[old].cities.push(City::new(old, position));
        game.build_wonder(wonder, position, old);
        game.players[old].construct(Academy, position, None);
        game.players[old].construct(Obelisk, position, None);

        game.players[old].victory_points(&game).assert_eq(8.0);

        conquer_city(&mut game, position, new, old);

        let c = game.players[new].get_city_mut(position);
        assert_eq!(1, c.player_index);
        assert_eq!(Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        old.victory_points(&game).assert_eq(4.0);
        new.victory_points(&game).assert_eq(5.0);
        assert_eq!(0, old.wonders_owned());
        assert_eq!(1, new.wonders_owned());
        assert_eq!(1, old.owned_buildings(&game));
        assert_eq!(1, new.owned_buildings(&game));
    }
}
