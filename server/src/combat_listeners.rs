use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat::{get_combat_round_end, Combat, CombatRetreatState};
use crate::consts::SHIP_CAPACITY;
use crate::content::builtin::{Builtin, BuiltinBuilder};
use crate::content::custom_phase_actions::{new_position_request, UnitsRequest};
use crate::game::Game;
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
    pub deny_tactics: Vec<usize>, // todo use effect when tactics cards are added
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
            deny_tactics: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CombatResult {
    AttackerWins,
    DefenderWins,
    Draw,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatEnd {
    pub result: CombatResult,
    pub combat: Combat,
}

impl CombatEnd {
    #[must_use]
    pub fn new(result: CombatResult, combat: Combat) -> Self {
        Self { result, combat }
    }

    #[must_use]
    pub fn is_attacker(&self, player: usize) -> bool {
        self.combat.attacker == player
    }

    #[must_use]
    pub fn is_defender(&self, player: usize) -> bool {
        self.combat.attacker != player
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
            self.combat.defender
        } else {
            self.combat.attacker
        }
    }

    #[must_use]
    pub fn captured_city(&self, player: usize, game: &Game) -> bool {
        self.is_attacker(player)
            && self.is_winner(player)
            && game
                .try_get_any_city(self.combat.defender_position)
                .is_some()
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
pub struct CombatRoundEnd {
    pub attacker_casualties: Casualties,
    pub defender_casualties: Casualties,
    #[serde(default)]
    pub can_retreat: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_result: Option<CombatResult>,
    pub combat: Combat,
}

impl CombatRoundEnd {
    #[must_use]
    pub fn new(
        attacker_casualties: Casualties,
        defender_casualties: Casualties,
        can_retreat: bool,
        combat: Combat,
        game: &Game,
    ) -> Self {
        let attackers_dead =
            combat.active_attackers(game).len() - attacker_casualties.fighters as usize == 0;
        let defenders_dead =
            combat.active_defenders(game).len() - defender_casualties.fighters as usize == 0;

        let final_result = if attackers_dead && defenders_dead {
            Some(CombatResult::Draw)
        } else if attackers_dead {
            Some(CombatResult::DefenderWins)
        } else if defenders_dead {
            Some(CombatResult::AttackerWins)
        } else {
            None
        };

        Self {
            attacker_casualties,
            defender_casualties,
            can_retreat,
            final_result,
            combat,
        }
    }
}

impl CombatRoundEnd {
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
        |game, player, c| c.fighting_units(game, player),
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
        |game, player, c| {
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
        |game, player, units, c| {
            kill_units(game, player, units, c);
            save_carried_units(units, game, player, c.position(player));
        },
    )
}

pub(crate) fn offer_retreat() -> Builtin {
    Builtin::builder("Offer Retreat", "Do you want to retreat?")
        .add_bool_request(
            |event| &mut event.on_combat_round_end,
            0,
            |game, player, r| {
                let c = &r.combat;
                if c.attacker == player && r.can_retreat {
                    let name = game.player_name(player);
                    game.add_info_log_item(&format!("{name} can retreat",));
                    Some("Do you want to retreat?".to_string())
                } else {
                    None
                }
            },
            |game, retreat| {
                let player_name = &retreat.player_name;
                if retreat.choice {
                    game.add_info_log_item(&format!("{player_name} retreats",));
                } else {
                    game.add_info_log_item(&format!("{player_name} does not retreat",));
                }
                if retreat.choice {
                    get_combat_round_end(game).combat.retreat =
                        CombatRetreatState::EndAfterCurrentRound;
                }
            },
        )
        .build()
}

pub(crate) fn choose_casualties(
    builder: BuiltinBuilder,
    priority: i32,
    get_casualties: impl Fn(&Casualties) -> u8 + 'static + Clone,
    get_choices: impl Fn(&Game, usize, &Combat) -> Vec<u32> + 'static + Clone,
    kill_units: impl Fn(&mut Game, usize, &[u32], &Combat) + 'static + Copy,
) -> Builtin {
    builder
        .add_units_request(
            |event| &mut event.on_combat_round_end,
            priority,
            move |game, player, r| {
                let c = &r.combat;

                let choices = get_choices(game, player, c).clone();

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
                    kill_units(game, player, &choices, c);
                    return None;
                }

                let first_type = p.get_unit(*choices.first().expect("no units")).unit_type;
                if choices
                    .iter()
                    .all(|u| p.get_unit(*u).unit_type == first_type)
                    || !p.is_human()
                {
                    game.add_info_log_item(&format!(
                        "{name} has to remove {casualties} of their {role} units",
                    ));
                    kill_units(game, player, &choices[..casualties as usize], c);
                    return None;
                }

                game.add_info_log_item(&format!(
                    "{name} has to remove {casualties} of their {role} units",
                ));
                Some(UnitsRequest::new(
                    player,
                    choices,
                    casualties..=casualties,
                    &format!("Remove {casualties} {role} units"),
                ))
            },
            move |game, s| {
                kill_units(game, s.player_index, &s.choice, &s.details.combat);
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
                && game.try_get_any_city(i.combat.defender_position).is_some()
                && !p.cities.is_empty()
                && p.available_units().settlers > 0
                && p.is_human()
            {
                let choices: Vec<Position> = p.cities.iter().map(|c| c.position).collect();
                Some(new_position_request(
                    choices,
                    1..=1,
                    "Select a city to place the free Settler Unit",
                ))
            } else {
                None
            }
        },
        |game, s| {
            let pos = s.choice[0];
            game.add_info_log_item(&format!(
                "{} gained 1 free Settler Unit at {} for losing a city",
                s.player_name, pos
            ));
            game.get_player_mut(s.player_index)
                .add_unit(pos, UnitType::Settler);
        },
    )
    .build()
}

fn kill_units(game: &mut Game, player: usize, killed_unit_ids: &[u32], c: &Combat) {
    let p = game.get_player(player);
    game.add_info_log_item(&format!(
        "{} removed {}",
        p.get_name(),
        killed_unit_ids
            .iter()
            .map(|id| p.get_unit(*id).unit_type)
            .collect::<Units>()
    ));

    let killer = c.opponent(player);

    for unit in killed_unit_ids {
        game.kill_unit(*unit, player, Some(killer));
        if player == c.attacker {
            get_combat_round_end(game)
                .combat
                .attackers
                .retain(|id| id != unit);
        }
    }
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
        let unit = game.players[player].get_unit_mut(unit.id);
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
