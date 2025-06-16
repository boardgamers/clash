use crate::ability_initializer::AbilityInitializerSetup;
use crate::combat::{Combat, CombatRetreatState, capture_position, log_round};
use crate::combat_roll::CombatHits;
use crate::combat_stats::CombatStats;
use crate::content::ability::{Ability, combat_event_origin};
use crate::content::persistent_events::{PersistentEventType, PositionRequest, UnitsRequest};
use crate::game::Game;
use crate::log::current_log_action_mut;
use crate::movement::move_units;
use crate::player::gain_unit;
use crate::player_events::{PersistentEvent, PersistentEvents};
use crate::position::Position;
use crate::tactics_card::{CombatRole, TacticsCard, TacticsCardTarget};
use crate::unit::{UnitType, kill_units};
use crate::utils;
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatStrength {
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub extra_dies: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "i8::is_zero")]
    pub extra_combat_value: i8,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    pub hit_cancels: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roll_log: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tactics_card: Option<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub deny_combat_abilities: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
    pub deny_tactics_card: bool,
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
            deny_combat_abilities: false,
            deny_tactics_card: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub enum CombatEventPhase {
    AllowTacticsCard,
    #[default]
    Default,
    RevealTacticsCard,
    TacticsCard,
    Done,
}

impl CombatEventPhase {
    #[must_use]
    pub fn is_default(&self) -> bool {
        matches!(self, CombatEventPhase::Default)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatRoundStart {
    pub combat: Combat,
    #[serde(default)]
    #[serde(skip_serializing_if = "CombatEventPhase::is_default")]
    pub phase: CombatEventPhase,
    pub attacker_strength: CombatStrength,
    pub defender_strength: CombatStrength,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_result: Option<CombatResult>,
}

impl CombatRoundStart {
    #[must_use]
    pub fn new(combat: Combat) -> Self {
        Self {
            combat,
            attacker_strength: CombatStrength::new(),
            defender_strength: CombatStrength::new(),
            phase: CombatEventPhase::AllowTacticsCard,
            final_result: None,
        }
    }

    #[must_use]
    pub fn is_active(&self, player: usize, action_card: u8, target: TacticsCardTarget) -> bool {
        target.is_active(
            player,
            &self.combat,
            action_card,
            self.attacker_strength.tactics_card.as_ref(),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum CombatResult {
    AttackerWins,
    DefenderWins,
    Draw,
}

impl CombatResult {
    #[must_use]
    pub fn winner(&self) -> Option<CombatRole> {
        match self {
            CombatResult::AttackerWins => Some(CombatRole::Attacker),
            CombatResult::DefenderWins => Some(CombatRole::Defender),
            CombatResult::Draw => None,
        }
    }
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
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CombatRoundEnd {
    #[serde(default)]
    #[serde(skip_serializing_if = "CombatEventPhase::is_default")]
    pub phase: CombatEventPhase,
    pub(crate) attacker: CombatHits,
    pub(crate) defender: CombatHits,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_result: Option<CombatResult>,
    pub combat: Combat,
}

impl CombatRoundEnd {
    #[must_use]
    pub(crate) fn new(attacker: CombatHits, defender: CombatHits, combat: Combat) -> Self {
        let mut combat_round_end = Self {
            attacker,
            defender,
            final_result: None,
            combat,
            phase: CombatEventPhase::TacticsCard,
        };
        combat_round_end.set_final_result();
        combat_round_end
    }

    pub(crate) fn can_retreat(&self) -> bool {
        self.combat.retreat == CombatRetreatState::CanRetreat
            && !self.attacker.all_opponents_killed()
            && !self.defender.all_opponents_killed()
    }

    fn set_final_result(&mut self) {
        let attackers_dead = self.defender.all_opponents_killed();
        let defenders_dead = self.attacker.all_opponents_killed();

        self.final_result = if attackers_dead && defenders_dead {
            Some(CombatResult::Draw)
        } else if attackers_dead {
            Some(CombatResult::DefenderWins)
        } else if defenders_dead {
            Some(CombatResult::AttackerWins)
        } else {
            None
        };
    }

    #[must_use]
    pub fn is_active(&self, player: usize, card: u8, target: TacticsCardTarget) -> bool {
        target.is_active(
            player,
            &self.combat,
            card,
            self.attacker.tactics_card.as_ref(),
        )
    }

    #[must_use]
    pub fn hits(&self, role: CombatRole) -> u8 {
        if role.is_attacker() {
            self.attacker.hits()
        } else {
            self.defender.hits()
        }
    }

    #[must_use]
    pub fn losses(&self, role: CombatRole) -> u8 {
        if role.is_attacker() {
            self.defender.hits()
        } else {
            self.attacker.hits()
        }
    }

    pub(crate) fn update_hits(
        &mut self,
        role: CombatRole,
        do_update: bool,
        update: impl Fn(&mut CombatHits),
    ) -> bool {
        let h = if role.is_attacker() {
            &mut self.attacker
        } else {
            &mut self.defender
        };
        if do_update {
            update(h);
            self.set_final_result();
            true
        } else {
            let mut copy = h.clone();
            update(&mut copy);
            h.hits() != copy.hits()
        }
    }

    #[must_use]
    pub fn role(&self, player: usize) -> CombatRole {
        self.combat.role(player)
    }
}

const ROUND_START_TYPES: &[CombatEventPhase; 4] = &[
    CombatEventPhase::AllowTacticsCard,
    CombatEventPhase::Default,
    CombatEventPhase::RevealTacticsCard,
    CombatEventPhase::TacticsCard,
];

pub(crate) fn combat_round_start(
    game: &mut Game,
    start: CombatRoundStart,
) -> Option<CombatRoundStart> {
    event_with_tactics(
        game,
        start,
        PersistentEventType::CombatRoundStart,
        ROUND_START_TYPES,
        |phase| match phase {
            CombatEventPhase::AllowTacticsCard => |e| &mut e.combat_round_start_allow_tactics,
            CombatEventPhase::Default => |e| &mut e.combat_round_start,
            CombatEventPhase::RevealTacticsCard => |e| &mut e.combat_round_start_reveal_tactics,
            CombatEventPhase::TacticsCard => |e| &mut e.combat_round_start_tactics,
            CombatEventPhase::Done => panic!("Invalid round type"),
        },
        |s| &mut s.phase,
        |s| &s.combat,
        |s| s.attacker_strength.tactics_card.as_ref(),
        |s| s.defender_strength.tactics_card.as_ref(),
    )
}

const ROUND_END_TYPES: &[CombatEventPhase; 2] =
    &[CombatEventPhase::TacticsCard, CombatEventPhase::Default];

pub(crate) fn combat_round_end(game: &mut Game, r: CombatRoundEnd) -> Option<Combat> {
    let e = event_with_tactics(
        game,
        r,
        PersistentEventType::CombatRoundEnd,
        ROUND_END_TYPES,
        |phase| match phase {
            CombatEventPhase::Default => |e| &mut e.combat_round_end,
            CombatEventPhase::TacticsCard => |e| &mut e.combat_round_end_tactics,
            _ => panic!("Invalid round type"),
        },
        |s| &mut s.phase,
        |s| &s.combat,
        |s| s.attacker.tactics_card.as_ref(),
        |s| s.defender.tactics_card.as_ref(),
    );
    let e = e?;

    if let Some(f) = &e.final_result {
        let c = e.combat;
        match f {
            CombatResult::AttackerWins => attacker_wins(game, c),
            CombatResult::DefenderWins => defender_wins(game, c),
            CombatResult::Draw => draw(game, c),
        }
        None
    } else if matches!(e.combat.retreat, CombatRetreatState::EndAfterCurrentRound) {
        None
    } else {
        let mut c = e.combat;
        c.stats.round += 1;
        log_round(game, &c);
        Some(c)
    }
}

fn attacker_wins(game: &mut Game, mut c: Combat) {
    game.add_info_log_item("Attacker wins");
    move_units(
        game,
        c.attacker(),
        &c.attackers,
        c.defender_position(),
        None,
    );
    capture_position(game, &mut c.stats);
    end_combat_and_store_stats(game, CombatEnd::new(CombatResult::AttackerWins, c));
}

fn defender_wins(game: &mut Game, c: Combat) {
    game.add_info_log_item("Defender wins");
    end_combat_and_store_stats(game, CombatEnd::new(CombatResult::DefenderWins, c));
}

pub(crate) fn draw(game: &mut Game, c: Combat) {
    if c.defender_fortress(game) && c.first_round() {
        game.add_info_log_item(&format!(
            "{} wins the battle because he has a defending fortress",
            game.player_name(c.defender())
        ));
        return end_combat_and_store_stats(game, CombatEnd::new(CombatResult::DefenderWins, c));
    }
    game.add_info_log_item("Battle ends in a draw");
    end_combat_and_store_stats(game, CombatEnd::new(CombatResult::Draw, c));
}

fn end_combat_and_store_stats(game: &mut Game, e: CombatEnd) {
    let mut stats = e.combat.stats;
    stats.result = Some(e.result.clone());
    end_combat(game, stats);
}

pub(crate) fn end_combat(game: &mut Game, s: CombatStats) {
    current_log_action_mut(game).combat_stats = Some(s.clone());
    on_end_combat(game, s);
}

pub(crate) fn on_end_combat(game: &mut Game, stats: CombatStats) {
    let _ = game.trigger_persistent_event(
        &[stats.attacker.player, stats.defender.player],
        |events| &mut events.combat_end,
        stats,
        PersistentEventType::CombatEnd,
    );
}

pub(crate) type GetCombatEvent<T> = fn(&mut PersistentEvents) -> &mut PersistentEvent<T>;

#[allow(clippy::too_many_arguments)]
pub(crate) fn event_with_tactics<T: Clone + PartialEq>(
    game: &mut Game,
    mut event_type: T,
    store_type: impl Fn(T) -> PersistentEventType + Clone + 'static + Sync + Send,
    round_types: &[CombatEventPhase],
    event: fn(&CombatEventPhase) -> GetCombatEvent<T>,
    get_round_type: impl Fn(&mut T) -> &mut CombatEventPhase,
    get_combat: impl Fn(&T) -> &Combat + Clone + 'static + Sync + Send,
    attacker_tactics_card: impl Fn(&T) -> Option<&u8>,
    defender_tactics_card: impl Fn(&T) -> Option<&u8>,
) -> Option<T> {
    let from = round_types
        .iter()
        .position(|t| t == get_round_type(&mut event_type))
        .expect("invalid round type");
    for t in round_types.iter().skip(from) {
        *get_round_type(&mut event_type) = t.clone();

        let store_type = store_type.clone();
        let get_combat = get_combat.clone();
        let event = event(t);
        let reveal_card = matches!(t, CombatEventPhase::RevealTacticsCard);

        event_type = (match t {
            CombatEventPhase::Default | CombatEventPhase::AllowTacticsCard => game
                .trigger_persistent_event(
                    &get_combat(&event_type).players(),
                    event,
                    event_type,
                    store_type,
                ),
            CombatEventPhase::RevealTacticsCard | CombatEventPhase::TacticsCard => {
                trigger_tactics_event(
                    game,
                    event_type,
                    event,
                    get_combat,
                    |s| attacker_tactics_card(s),
                    |s| defender_tactics_card(s),
                    store_type,
                    reveal_card,
                )
            }
            CombatEventPhase::Done => panic!("Invalid round type"),
        })?;
    }
    *get_round_type(&mut event_type) = CombatEventPhase::Done;
    Some(event_type)
}

pub(crate) fn trigger_tactics_event<T>(
    game: &mut Game,
    event_type: T,
    event: fn(&mut PersistentEvents) -> &mut PersistentEvent<T>,
    get_combat: impl Fn(&T) -> &Combat,
    get_attacker_tactics_card: impl Fn(&T) -> Option<&u8>,
    get_defender_tactics_card: impl Fn(&T) -> Option<&u8>,
    store_type: impl Fn(T) -> PersistentEventType,
    reveal_card: bool,
) -> Option<T>
where
    T: Clone + PartialEq,
{
    let attacker_card = |event_type: &T, game: &Game| -> Option<TacticsCard> {
        get_attacker_tactics_card(event_type).map(move |c| game.cache.get_tactics_card(*c).clone())
    };
    let defender_card = |event_type: &T, game: &Game| -> Option<TacticsCard> {
        get_defender_tactics_card(event_type).map(move |c| game.cache.get_tactics_card(*c).clone())
    };

    if get_attacker_tactics_card(&event_type).is_none()
        && get_defender_tactics_card(&event_type).is_none()
    {
        return Some(event_type);
    }

    let combat = get_combat(&event_type);

    add_tactics_listener(
        game,
        reveal_card,
        attacker_card(&event_type, game),
        combat,
        CombatRole::Attacker,
    );
    add_tactics_listener(
        game,
        reveal_card,
        defender_card(&event_type, game),
        combat,
        CombatRole::Defender,
    );

    let players = &combat.players();
    let result = game.trigger_persistent_event(players, event, event_type.clone(), store_type);

    if let Some(card) = attacker_card(&event_type, game) {
        let card = card.clone();
        for p in players {
            card.listeners.deinit(game, *p);
        }
    }
    if let Some(card) = defender_card(&event_type, game) {
        let card = card.clone();
        for p in players {
            card.listeners.deinit(game, *p);
        }
    }

    result
}

fn add_tactics_listener(
    game: &mut Game,
    reveal_card: bool,
    card: Option<TacticsCard>,
    combat: &Combat,
    role: CombatRole,
) {
    let players = &combat.players();

    if let Some(card) = card {
        for p in players {
            match role {
                // avoid clash in priority - and attacker card requests should come first anyway
                CombatRole::Attacker => card.listeners.init_with_prio_delta(game, *p, 100),
                CombatRole::Defender => card.listeners.init(game, *p),
            }
        }
        if reveal_card {
            game.add_info_log_item(&format!(
                "{} reveals Tactics Card {}",
                game.player_name(combat.player(role)),
                card.name
            ));
        }
    }
}

pub(crate) fn choose_fighter_casualties() -> Ability {
    Ability::builder("Choose Casualties", "Choose which carried units to remove.")
        .add_units_request(
            |event| &mut event.combat_round_end,
            1,
            move |game, player, r| {
                let choices = r.combat.fighting_units(game, player.index).clone();

                let role = r.role(player.index);
                let role_str = if role.is_attacker() {
                    "attacking"
                } else {
                    "defending"
                };
                let casualties = r.losses(role);
                if casualties == 0 {
                    return None;
                }
                let p = game.player(player.index);
                if casualties == choices.len() as u8 {
                    player.log(game, &format!("Remove all {role_str} units",));
                    let player1 = player.index;
                    let c = &mut r.combat;
                    kill_combat_units(game, c, player1, &choices);
                    return None;
                }

                let first_type = p.get_unit(*choices.first().expect("no units")).unit_type;
                if choices
                    .iter()
                    .all(|u| p.get_unit(*u).unit_type == first_type)
                    || !p.is_human()
                {
                    player.log(
                        game,
                        &format!("Remove {casualties} of their {role_str} units",),
                    );
                    let player1 = player.index;
                    let killed_unit_ids = &choices[..casualties as usize];
                    let c = &mut r.combat;
                    kill_combat_units(game, c, player1, killed_unit_ids);
                    return None;
                }

                player.log(
                    game,
                    &format!("Remove {casualties} of their {role_str} units",),
                );
                Some(UnitsRequest::new(
                    player.index,
                    choices,
                    casualties..=casualties,
                    &format!("Remove {casualties} {role_str} units"),
                ))
            },
            move |game, s, e| {
                let player = s.player_index;
                let killed_unit_ids = &s.choice;
                let c = &mut e.combat;
                kill_combat_units(game, c, player, killed_unit_ids);
            },
        )
        .build()
}

pub(crate) fn offer_retreat() -> Ability {
    Ability::builder("Offer Retreat", "Do you want to retreat?")
        .add_bool_request(
            |event| &mut event.combat_round_end,
            0,
            |game, p, r| {
                let player = p.index;
                let c = &r.combat;
                if c.attacker() == player && r.can_retreat() {
                    let name = game.player_name(player);
                    p.log(game, &format!("{name} can retreat",));
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

pub(crate) fn place_settler() -> Ability {
    Ability::builder(
        "Place Settler",
        "After losing a city, place a settler in another city.",
    )
    .add_position_request(
        |event| &mut event.combat_end,
        102,
        |game, p, i| {
            let player_index = p.index;
            let p = game.player(player_index);
            if i.is_defender(player_index)
                && i.is_loser(player_index)
                && game.try_get_any_city(i.defender.position).is_some()
                && !p.cities.is_empty()
                && p.available_units().settlers > 0
                && p.is_human()
            {
                let choices: Vec<Position> = p.cities.iter().map(|c| c.position).collect();
                let needed = 1..=1;
                Some(PositionRequest::new(
                    choices,
                    needed,
                    "Select a city to place the free Settler Unit",
                ))
            } else {
                None
            }
        },
        |game, s, _e| {
            let pos = s.choice[0];
            s.log(
                game,
                &format!("Gain 1 free Settler Unit at {pos} for losing a city",),
            );
            gain_unit(game, s.player_index, pos, UnitType::Settler, &s.origin);
        },
    )
    .build()
}

pub(crate) fn kill_combat_units(
    game: &mut Game,
    c: &mut Combat,
    player: usize,
    killed_unit_ids: &[u32],
) {
    kill_units_with_stats(&mut c.stats, game, player, killed_unit_ids);
    for unit in killed_unit_ids {
        if player == c.attacker() {
            c.attackers.retain(|id| id != unit);
        }
    }
}

pub(crate) fn kill_units_with_stats(
    stats: &mut CombatStats,
    game: &mut Game,
    player: usize,
    killed_unit_ids: &[u32],
) {
    if killed_unit_ids.is_empty() {
        return;
    }

    let p = game.player(player);
    stats.player_mut(stats.role(player)).add_losses(
        &killed_unit_ids
            .iter()
            .map(|id| p.get_unit(*id).unit_type)
            .collect_vec(),
    );
    kill_units(
        game,
        killed_unit_ids,
        player,
        Some(stats.opponent(player).player),
        &combat_event_origin(),
    );
}
