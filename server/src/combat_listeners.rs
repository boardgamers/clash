use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat::{get_combat, take_combat, Combat};
use crate::consts::SHIP_CAPACITY;
use crate::content::builtin::{Builtin, BuiltinBuilder};
use crate::content::custom_phase_actions::{PositionRequest, UnitsRequest};
use crate::game::{Game, GameState};
use crate::position::Position;
use crate::unit::{UnitType, Units};
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub defender: usize,
}

impl CombatResultInfo {
    #[must_use]
    pub fn new(
        result: CombatResult,
        attacker: usize,
        defender: usize,
        defender_position: Position,
    ) -> Self {
        Self {
            result,
            defender_position,
            attacker,
            defender,
        }
    }

    #[must_use]
    pub fn is_attacker(&self, player: usize) -> bool {
        self.attacker == player
    }

    #[must_use]
    pub fn is_defender(&self, player: usize) -> bool {
        self.attacker != player
    }

    #[must_use]
    pub fn is_loser(&self, player: usize) -> bool {
        if self.is_attacker(player) {
            self.result == CombatResult::DefenderWins
        } else {
            self.result == CombatResult::AttackerWins
        }
    }

    #[must_use]
    pub fn is_winner(&self, player: usize) -> bool {
        if self.is_attacker(player) {
            self.result == CombatResult::AttackerWins
        } else {
            self.result == CombatResult::DefenderWins
        }
    }

    #[must_use]
    pub fn opponent(&self, player: usize) -> usize {
        if self.is_attacker(player) {
            self.defender
        } else {
            self.attacker
        }
    }

    #[must_use]
    pub fn captured_city(&self, player: usize, game: &Game) -> bool {
        self.is_attacker(player)
            && self.is_winner(player)
            && game.get_any_city(self.defender_position).is_some()
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
            |game, player, r| {
                let c = get_combat(game);
                if c.attacker == player && r.can_retreat {
                    let p = game.get_player(player);
                    let name = p.get_name();
                    game.add_info_log_item(&format!("{name} can retreat",));
                    true
                } else {
                    false
                }
            },
            |game, retreat| {
                let player_name = &retreat.player_name;
                if retreat.choice {
                    game.add_info_log_item(&format!("{player_name} retreats",));
                } else {
                    game.add_info_log_item(&format!("{player_name} does not retreat",));
                }
                let mut c = take_combat(game);
                c.round_result.as_mut().expect("no round result").retreated = retreat.choice;
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
            move |game, player, r| {
                let c = get_combat(game);

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
                Some(UnitsRequest::new(
                    player,
                    choices,
                    casualties,
                    Some(format!("Remove {casualties} {role} units")),
                ))
            },
            move |game, s| {
                kill_units(game, s.player_index, &s.choice);
            },
        )
        .build()
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
            let p = game.get_player(player_index);
            if i.is_defender(player_index)
                && i.is_loser(player_index)
                && game.get_any_city(i.defender_position).is_some()
                && !p.cities.is_empty()
                && p.available_units().settlers > 0
                && p.is_human()
            {
                let choices: Vec<Position> = p.cities.iter().map(|c| c.position).collect();
                Some(PositionRequest::new(choices, None))
            } else {
                None
            }
        },
        |game, s| {
            game.add_info_log_item(&format!(
                "{} gained 1 free Settler Unit at {} for losing a city",
                s.player_name, s.choice
            ));
            game.get_player_mut(s.player_index)
                .add_unit(s.choice, UnitType::Settler);
        },
    )
    .build()
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
    let killer = c.opponent(player);

    for unit in killed_unit_ids {
        game.kill_unit(*unit, player, Some(killer));
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
