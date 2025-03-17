use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat::{capture_position, Combat, CombatRetreatState};
use crate::consts::SHIP_CAPACITY;
use crate::content::builtin::{Builtin, BuiltinBuilder};
use crate::content::custom_phase_actions::{new_position_request, CurrentEventType, UnitsRequest};
use crate::content::tactics_cards;
use crate::game::Game;
use crate::movement::move_units;
use crate::player_events::{CurrentEvent, PlayerEvents};
use crate::position::Position;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units};
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatStrength {
    pub extra_dies: u8,
    pub extra_combat_value: u8,
    pub hit_cancels: u8,
    pub roll_log: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics_card: Option<String>,
}

impl Default for CombatStrength {
    fn default() -> Self {
        Self::new()
    }
}

impl CombatStrength {
    #[must_use]
    pub fn new() -> Self {
        Self {
            extra_dies: 0,
            extra_combat_value: 0,
            hit_cancels: 0,
            roll_log: vec![],
            tactics_card: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CombatRoundType {
    Default,
    TacticsCardAttacker,
    TacticsCardDefender,
    Done,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatRoundStart {
    pub combat: Combat,
    pub round_type: CombatRoundType,
    pub attacker_strength: CombatStrength,
    pub defender_strength: CombatStrength,
}

impl CombatRoundStart {
    #[must_use]
    pub fn new(combat: Combat) -> Self {
        Self {
            combat,
            attacker_strength: CombatStrength::new(),
            defender_strength: CombatStrength::new(),
            round_type: CombatRoundType::Default,
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics_card: Option<String>,
}

impl Casualties {
    #[must_use]
    pub fn new(
        fighters: u8,
        game: &Game,
        c: &Combat,
        player: usize,
        tactics_card: Option<String>,
    ) -> Self {
        Self {
            fighters,
            carried_units: c.carried_units_casualties(game, player, fighters),
            tactics_card,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatRoundEnd {
    pub round_type: CombatRoundType,
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
            round_type: CombatRoundType::Default,
        }
    }
}

impl CombatRoundEnd {
    #[must_use]
    pub fn casualties(&self, role: CombatRole) -> &Casualties {
        if role.is_attacker() {
            &self.attacker_casualties
        } else {
            &self.defender_casualties
        }
    }
}

const ROUND_START_TYPES: &[CombatRoundType; 3] = &[
    CombatRoundType::Default,
    CombatRoundType::TacticsCardAttacker,
    CombatRoundType::TacticsCardDefender,
];

pub(crate) fn combat_round_start(
    game: &mut Game,
    start: &CombatRoundStart,
) -> Option<CombatRoundStart> {
    event_with_tactics(
        game,
        start,
        CurrentEventType::CombatRoundStart,
        ROUND_START_TYPES,
        |events| &mut events.on_combat_round_start,
        |events| &mut events.on_combat_round_start_tactics,
        |s| &mut s.round_type,
        |s| &s.combat,
        |s| s.attacker_strength.tactics_card.as_ref(),
        |s| s.defender_strength.tactics_card.as_ref(),
    )
}

const ROUND_END_TYPES: &[CombatRoundType; 3] = &[
    CombatRoundType::TacticsCardAttacker,
    CombatRoundType::TacticsCardDefender,
    CombatRoundType::Default,
];

pub(crate) fn combat_round_end(game: &mut Game, r: &CombatRoundEnd) -> Option<Combat> {
    let e = event_with_tactics(
        game,
        r,
        CurrentEventType::CombatRoundEnd,
        ROUND_END_TYPES,
        |events| &mut events.on_combat_round_end,
        |events| &mut events.on_combat_round_end_tactics,
        |s| &mut s.round_type,
        |s| &s.combat,
        |s| s.attacker_casualties.tactics_card.as_ref(),
        |s| s.defender_casualties.tactics_card.as_ref(),
    );
    let mut c = match e {
        None => return None,
        Some(e) => e.combat,
    };

    if let Some(f) = &r.final_result {
        match f {
            CombatResult::AttackerWins => attacker_wins(game, c),
            CombatResult::DefenderWins => defender_wins(game, c),
            CombatResult::Draw => draw(game, c),
        }
        None
    } else if matches!(c.retreat, CombatRetreatState::EndAfterCurrentRound) {
        None
    } else {
        c.round += 1;
        crate::combat::log_round(game, &c);
        Some(c)
    }
}

fn attacker_wins(game: &mut Game, c: Combat) {
    game.add_info_log_item("Attacker wins");
    move_units(game, c.attacker, &c.attackers, c.defender_position, None);
    capture_position(game, c.defender, c.defender_position, c.attacker);
    end_combat(game, &CombatEnd::new(CombatResult::AttackerWins, c));
}

fn defender_wins(game: &mut Game, c: Combat) {
    game.add_info_log_item("Defender wins");
    end_combat(game, &CombatEnd::new(CombatResult::DefenderWins, c));
}

pub(crate) fn draw(game: &mut Game, c: Combat) {
    if c.defender_fortress(game) && c.round == 1 {
        game.add_info_log_item(&format!(
            "{} wins the battle because he has a defending fortress",
            game.player_name(c.defender)
        ));
        return end_combat(game, &CombatEnd::new(CombatResult::DefenderWins, c));
    }
    game.add_info_log_item("Battle ends in a draw");
    end_combat(game, &CombatEnd::new(CombatResult::Draw, c));
}

pub(crate) fn end_combat(game: &mut Game, info: &CombatEnd) {
    let c = &info.combat;

    game.trigger_current_event(
        &c.players(),
        |events| &mut events.on_combat_end,
        info,
        CurrentEventType::CombatEnd,
        None,
    );
}

pub(crate) fn event_with_tactics<T: Clone + PartialEq>(
    game: &mut Game,
    event_type: &T,
    store_type: impl Fn(T) -> CurrentEventType + Clone + 'static,
    round_types: &[CombatRoundType; 3],
    event: fn(&mut PlayerEvents) -> &mut CurrentEvent<T>,
    tactics_event: fn(&mut PlayerEvents) -> &mut CurrentEvent<T>,
    get_round_type: impl Fn(&mut T) -> &mut CombatRoundType,
    get_combat: impl Fn(&T) -> &Combat + Clone + 'static,
    attacker_tactics_card: impl Fn(&T) -> Option<&String>,
    defender_tactics_card: impl Fn(&T) -> Option<&String>,
) -> Option<T> {
    let mut s = event_type.clone();
    let from = round_types
        .iter()
        .position(|t| t == get_round_type(&mut s))
        .expect("invalid round type");
    for t in round_types.iter().skip(from) {
        *get_round_type(&mut s) = t.clone();

        let store_type = store_type.clone();
        let get_combat = get_combat.clone();

        let option = match t {
            CombatRoundType::Default => {
                game.trigger_current_event(&get_combat(&s).players(), event, &s, store_type, None)
            }
            CombatRoundType::TacticsCardAttacker => trigger_tactics_event(
                game,
                &s,
                tactics_event,
                get_combat,
                |s| attacker_tactics_card(s),
                store_type,
            ),
            CombatRoundType::TacticsCardDefender => trigger_tactics_event(
                game,
                &s,
                tactics_event,
                get_combat,
                |s| defender_tactics_card(s),
                store_type,
            ),
            CombatRoundType::Done => panic!("Invalid round type"),
        };
        s = option?;
    }
    *get_round_type(&mut s) = CombatRoundType::Done;
    Some(s)
}

pub(crate) fn trigger_tactics_event<T>(
    game: &mut Game,
    event_type: &T,
    event: fn(&mut PlayerEvents) -> &mut CurrentEvent<T>,
    get_combat: impl Fn(&T) -> &Combat,
    get_tactics_card: impl Fn(&T) -> Option<&String>,
    store_type: impl Fn(T) -> CurrentEventType,
) -> Option<T>
where
    T: Clone + PartialEq,
{
    game.trigger_current_event_with_listener(
        &get_combat(event_type).players(),
        event,
        get_tactics_card(event_type)
            .as_ref()
            .map(|c| tactics_cards::get_tactics_card(c).listeners)
            .as_ref(),
        event_type,
        store_type,
        None,
    )
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
            |game, retreat, e| {
                let player_name = &retreat.player_name;
                if retreat.choice {
                    game.add_info_log_item(&format!("{player_name} retreats",));
                } else {
                    game.add_info_log_item(&format!("{player_name} does not retreat",));
                }
                if retreat.choice {
                    e.combat.retreat = CombatRetreatState::EndAfterCurrentRound;
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
    kill_units: impl Fn(&mut Game, usize, &[u32], &mut Combat) + 'static + Copy,
) -> Builtin {
    builder
        .add_units_request(
            |event| &mut event.on_combat_round_end,
            priority,
            move |game, player, r| {
                let choices = get_choices(game, player, &mut r.combat).clone();

                let role = r.combat.role(player);
                let role_str = if role.is_attacker() {
                    "attacking"
                } else {
                    "defending"
                };
                let casualties = get_casualties(r.casualties(role));
                if casualties == 0 {
                    return None;
                }
                let p = game.get_player(player);
                let name = p.get_name();
                if casualties == choices.len() as u8 {
                    game.add_info_log_item(&format!(
                        "{name} has to remove all of their {role_str} units",
                    ));
                    kill_units(game, player, &choices, &mut r.combat);
                    return None;
                }

                let first_type = p.get_unit(*choices.first().expect("no units")).unit_type;
                if choices
                    .iter()
                    .all(|u| p.get_unit(*u).unit_type == first_type)
                    || !p.is_human()
                {
                    game.add_info_log_item(&format!(
                        "{name} has to remove {casualties} of their {role_str} units",
                    ));
                    kill_units(game, player, &choices[..casualties as usize], &mut r.combat);
                    return None;
                }

                game.add_info_log_item(&format!(
                    "{name} has to remove {casualties} of their {role_str} units",
                ));
                Some(UnitsRequest::new(
                    player,
                    choices,
                    casualties..=casualties,
                    &format!("Remove {casualties} {role_str} units"),
                ))
            },
            move |game, s, e| {
                kill_units(game, s.player_index, &s.choice, &mut e.combat);
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
        |game, s, _e| {
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

fn kill_units(game: &mut Game, player: usize, killed_unit_ids: &[u32], c: &mut Combat) {
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
            c.attackers.retain(|id| id != unit);
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
