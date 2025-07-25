use crate::barbarians::get_barbarians_player;
use crate::city::MoodState::Angry;
use crate::city::{City, gain_city, lose_city, raze_city, set_city_mood};
use crate::city_pieces::{Building, gain_building, lose_building};
use crate::combat_listeners::{
    CombatEventPhase, CombatResult, CombatRoundEnd, CombatRoundStart, CombatStrength,
    combat_round_end, combat_round_start, end_combat, kill_units_with_stats,
};
use crate::combat_roll::{CombatHits, CombatRoundStats};
use crate::combat_stats;
use crate::combat_stats::{CombatStats, active_defenders, new_combat_stats};
use crate::content::ability::combat_event_origin;
use crate::content::persistent_events::PersistentEventType;
use crate::events::{EventOrigin, EventPlayer};
use crate::game::Game;
use crate::movement::{MoveUnits, MovementRestriction, move_units, stop_current_move};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units, carried_units};
use crate::wonder::{Wonder, gain_wonder, init_wonder, lose_wonder};
use combat_stats::active_attackers;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum CombatModifier {
    CancelFortressExtraDie,
    CancelFortressIgnoreHit,
    SteelWeaponsAttacker,
    SteelWeaponsDefender,
    TrojanHorse,
    GreatWarlord,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Copy)]
pub enum CombatRetreatState {
    CanRetreat,
    CannotRetreat,
    EndAfterCurrentRound,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Combat {
    pub attackers: Vec<u32>,
    pub retreat: CombatRetreatState,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub modifiers: Vec<CombatModifier>,
    pub stats: CombatStats,
}

impl Combat {
    #[must_use]
    pub fn new(attackers: Vec<u32>, can_retreat: bool, stats: CombatStats) -> Self {
        Self {
            attackers,
            modifiers: vec![],
            retreat: if can_retreat {
                CombatRetreatState::CanRetreat
            } else {
                CombatRetreatState::CannotRetreat
            },
            stats,
        }
    }

    #[must_use]
    pub fn first_round(&self) -> bool {
        self.stats.round == 1
    }

    #[must_use]
    pub fn attacker(&self) -> usize {
        self.stats.attacker.player
    }

    #[must_use]
    pub fn defender(&self) -> usize {
        self.stats.defender.player
    }

    #[must_use]
    pub fn defender_position(&self) -> Position {
        self.stats.defender.position
    }

    #[must_use]
    pub fn fighting_units(&self, game: &Game, player: usize) -> Vec<u32> {
        if player == self.attacker() {
            self.active_attackers(game)
        } else {
            self.active_defenders(game)
        }
    }

    #[must_use]
    pub(crate) fn active_attackers(&self, game: &Game) -> Vec<u32> {
        active_attackers(
            game,
            self.attacker(),
            &self.attackers,
            self.defender_position(),
        )
    }

    #[must_use]
    pub fn active_defenders(&self, game: &Game) -> Vec<u32> {
        active_defenders(game, self.defender(), self.defender_position())
    }

    #[must_use]
    pub fn defender_fortress(&self, game: &Game) -> bool {
        self.defender_city(game)
            .is_some_and(|city| city.pieces.fortress.is_some())
    }

    #[must_use]
    pub fn defender_city<'a>(&self, game: &'a Game) -> Option<&'a City> {
        game.players[self.defender()].try_get_city(self.defender_position())
    }

    #[must_use]
    pub fn defender_temple(&self, game: &Game) -> bool {
        self.defender_city(game)
            .is_some_and(|city| city.pieces.temple.is_some())
    }

    #[must_use]
    pub fn is_sea_battle(&self, game: &Game) -> bool {
        game.map.is_sea(self.defender_position())
    }

    #[must_use]
    pub fn is_land_battle(&self, game: &Game) -> bool {
        game.map.is_land(self.defender_position())
    }

    #[must_use]
    pub fn is_land_battle_with_leader(&self, role: CombatRole, game: &Game) -> bool {
        self.is_land_battle(game) && self.has_leader(role, game)
    }

    #[must_use]
    pub fn is_disembarking_attacker(&self, role: CombatRole, game: &Game) -> bool {
        role.is_attacker() && game.map.is_sea(self.stats.attacker.position)
    }

    #[must_use]
    pub fn opponent(&self, player: usize) -> usize {
        if player == self.attacker() {
            self.defender()
        } else {
            self.attacker()
        }
    }

    #[must_use]
    pub fn opponent_role(&self, player: usize) -> CombatRole {
        if player == self.attacker() {
            CombatRole::Defender
        } else {
            CombatRole::Attacker
        }
    }

    #[must_use]
    pub fn players(&self) -> [usize; 2] {
        [self.attacker(), self.defender()]
    }

    #[must_use]
    pub fn role(&self, player: usize) -> CombatRole {
        self.stats.role(player)
    }

    #[must_use]
    pub fn player(&self, role: CombatRole) -> usize {
        match role {
            CombatRole::Attacker => self.attacker(),
            CombatRole::Defender => self.defender(),
        }
    }

    #[must_use]
    pub fn has_leader(&self, role: CombatRole, game: &Game) -> bool {
        let p = self.player(role);
        self.fighting_units(game, p)
            .iter()
            .any(|&unit_id| game.player(p).get_unit(unit_id).is_leader())
    }

    #[must_use]
    pub fn is_barbarian_battle(&self, role: CombatRole, game: &Game) -> bool {
        self.opponent(self.player(role)) == get_barbarians_player(game).index
    }
}

pub fn initiate_combat(
    game: &mut Game,
    defender: usize,
    defender_position: Position,
    attacker: usize,
    attackers: Vec<u32>,
    can_retreat: bool,
) {
    let stats = new_combat_stats(
        game,
        defender,
        defender_position,
        attacker,
        &attackers,
        None,
    );
    let combat = Combat::new(attackers, can_retreat, stats);
    log_round(game, &combat);
    start_combat(game, combat);
}

pub(crate) fn log_round(game: &mut Game, c: &Combat) {
    game.add_info_log_group(format!("Combat round {}", c.stats.round));
    let origin = &combat_event_origin();
    game.log(
        c.attacker(),
        origin,
        &format!(
            "Attacking with {}",
            c.attackers
                .iter()
                .flat_map(|u| {
                    let p = game.player(c.attacker());
                    let u = p.get_unit(*u);
                    vec![u.unit_type]
                        .into_iter()
                        .chain(
                            carried_units(u.id, p)
                                .iter()
                                .map(|u| p.get_unit(*u).unit_type),
                        )
                        .collect_vec()
                })
                .collect::<Units>()
                .to_string(Some(game))
        ),
    );
    game.log(
        c.defender(),
        origin,
        &format!(
            "Defending with {}",
            game.player(c.defender())
                .get_units(c.defender_position())
                .iter()
                .map(|u| u.unit_type)
                .collect::<Units>()
                .to_string(Some(game))
        ),
    );
}

pub(crate) fn start_combat(game: &mut Game, combat: Combat) {
    stop_current_move(game);

    let c = match game.trigger_persistent_event(
        &combat.players(),
        |events| &mut events.combat_start,
        combat,
        PersistentEventType::CombatStart,
    ) {
        None => return,
        Some(c) => c,
    };

    combat_loop(game, CombatRoundStart::new(c));
}

pub(crate) fn get_combat_strength(player: usize, e: &CombatRoundStart) -> &CombatStrength {
    if player == e.combat.attacker() {
        &e.attacker_strength
    } else if player == e.combat.defender() {
        &e.defender_strength
    } else {
        panic!("Invalid player index")
    }
}

pub(crate) fn update_combat_strength(
    game: &mut Game,
    player: usize,
    e: &mut CombatRoundStart,
    update: impl Fn(&mut Game, &Combat, &mut CombatStrength, CombatRole) + Clone + Sync + Send,
) {
    if player == e.combat.attacker() {
        update(
            game,
            &e.combat,
            &mut e.attacker_strength,
            CombatRole::Attacker,
        );
    } else if player == e.combat.defender() {
        update(
            game,
            &e.combat,
            &mut e.defender_strength,
            CombatRole::Defender,
        );
    } else {
        panic!("Invalid player index")
    }
}

pub(crate) fn combat_loop(game: &mut Game, mut s: CombatRoundStart) {
    loop {
        if s.phase != CombatEventPhase::Done {
            match combat_round_start(game, s) {
                Some(value) => s = value,
                None => return,
            }
        }

        let c = s.combat;

        let a_t = s.attacker_strength.tactics_card.take();
        let d_t = s.defender_strength.tactics_card.take();

        let result = if let Some(result) = s.final_result {
            let mut round_end = CombatRoundEnd::new(
                CombatHits::new(None, 0, 0, 0),
                CombatHits::new(None, 0, 0, 0),
                c,
            );
            round_end.final_result = Some(result);
            round_end.phase = CombatEventPhase::Default;
            round_end
        } else {
            let mut a = CombatRoundStats::roll(c.attacker(), &c, game, s.attacker_strength);
            let mut d = CombatRoundStats::roll(c.defender(), &c, game, s.defender_strength);

            CombatRoundEnd::new(
                a.determine_hits(&d, game, a_t),
                d.determine_hits(&a, game, d_t),
                c,
            )
        };

        if let Some(r) = combat_round_end(game, result) {
            s = CombatRoundStart::new(r);
        } else {
            return;
        }
    }
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
    attacker: &EventPlayer,
    defender: &EventPlayer,
) {
    let attacker_is_human = attacker.get(game).is_human();
    let d = defender.get(game);
    let size = d.get_city(position).mood_modified_size(d);
    if attacker_is_human && defender.get(game).is_human() {
        attacker.gain_resources(game, ResourcePile::gold(size as u8));
    }

    if attacker.get(game).is_city_available() {
        let city = lose_city(game, defender, position);
        gain_city(game, attacker, city);
        if attacker_is_human {
            take_over_city(game, position, attacker, defender);
        }
        set_city_mood(game, position, &attacker.origin, Angry);
    } else {
        if attacker_is_human {
            attacker
                .with_origin(EventOrigin::Ability("Raze captured city".to_string()))
                .gain_resources(
                    game,
                    ResourcePile::gold(defender.get(game).get_city(position).size() as u8),
                );
        }
        raze_city(game, defender, position);
    }
}

fn take_over_city(
    game: &mut Game,
    position: Position,
    attacker: &EventPlayer,
    defender: &EventPlayer,
) {
    let pieces = game
        .player(attacker.index)
        .get_city(position)
        .pieces
        .clone();
    for wonder in pieces.wonders.clone() {
        lose_wonder(game, defender, wonder, position);
        gain_wonder(game, attacker, wonder, position);
        init_wonder(game, attacker.index, wonder);
    }

    for (building, owner) in pieces.building_owners() {
        if matches!(building, Building::Obelisk) {
            continue;
        }
        let Some(owner) = owner else {
            continue;
        };
        if owner != defender.index {
            continue;
        }
        if attacker.get(game).is_building_available(building, game) {
            gain_building(game, attacker, building, position);
        } else {
            lose_building(game, defender, building, position);
            attacker.gain_resources(game, ResourcePile::gold(1));
        }
    }
}

pub(crate) fn capture_position(game: &mut Game, stats: &mut CombatStats) {
    let p = &EventPlayer::from_player(stats.attacker.player, game, combat_event_origin());
    let old_player = stats.defender.player;
    let position = stats.defender.position;
    let captured_settlers = game.players[old_player]
        .get_units(position)
        .iter()
        .map(|unit| unit.id)
        .collect_vec();
    if !captured_settlers.is_empty() {
        p.log(
            game,
            &format!(
                "Kill {} settlers of {}",
                captured_settlers.len(),
                game.player_name(old_player)
            ),
        );
    }
    kill_units_with_stats(stats, game, old_player, &captured_settlers);
    if game.player(old_player).try_get_city(position).is_some() {
        let d = &EventPlayer::from_player(old_player, game, combat_event_origin());
        conquer_city(game, position, p, d);
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum MoveResult {
    Combat,
    Move,
}

fn move_to_enemy_player_tile(
    game: &mut Game,
    player_index: usize,
    unit_ids: &Vec<u32>,
    destination: Position,
    defender: usize,
) -> MoveResult {
    let has_defending_units = game
        .player(defender)
        .get_units(destination)
        .iter()
        .any(|unit| unit.is_military());
    let city = game.player(defender).try_get_city(destination);
    let has_fortress = city.is_some_and(|city| city.pieces.fortress.is_some());

    if game.player(player_index).is_human() {
        apply_battle_movement_restriction(game, player_index, unit_ids);
    } else if city.is_some()
        && game
            .player(defender)
            .wonders_owned
            .contains(Wonder::GreatWall)
    {
        game.log(
            defender,
            &EventOrigin::Wonder(Wonder::GreatWall),
            "Automatic win against Barbarians",
        );

        let mut s = new_combat_stats(
            game,
            defender,
            destination,
            player_index,
            unit_ids,
            Some(CombatResult::DefenderWins),
        );
        kill_units_with_stats(&mut s, game, player_index, unit_ids);
        end_combat(game, s);
        return MoveResult::Combat;
    }

    if has_defending_units || has_fortress {
        initiate_combat(
            game,
            defender,
            destination,
            player_index,
            unit_ids.clone(),
            game.player(player_index).is_human(),
        );
        return MoveResult::Combat;
    }
    MoveResult::Move
}

fn apply_battle_movement_restriction(game: &mut Game, player_index: usize, unit_ids: &Vec<u32>) {
    let mut military = false;
    let can_ignore_battle_movement_restrictions_for_ships = game
        .player(player_index)
        .has_special_advance(SpecialAdvance::Longships);
    let mut used_longships = false;

    for unit_id in unit_ids {
        let unit = game.player_mut(player_index).get_unit_mut(*unit_id);
        if unit.is_military() {
            military = true;
            if unit.is_ship() && can_ignore_battle_movement_restrictions_for_ships {
                used_longships = true;
                continue;
            }

            if unit
                .movement_restrictions
                .contains(&MovementRestriction::Battle)
            {
                panic!("unit can't attack");
            }
            unit.movement_restrictions.push(MovementRestriction::Battle);
        }
    }

    if used_longships {
        EventPlayer::from_player(
            player_index,
            game,
            EventOrigin::SpecialAdvance(SpecialAdvance::Longships),
        )
        .log(game, "Ignore battle movement restrictions");
    }

    assert!(military, "Need military units to attack");
}

pub(crate) fn move_with_possible_combat(game: &mut Game, player_index: usize, m: &MoveUnits) {
    let enemy = game.enemy_player(player_index, m.destination);
    if let Some(defender) = enemy {
        if move_to_enemy_player_tile(game, player_index, &m.units, m.destination, defender)
            == MoveResult::Combat
        {
            return;
        }

        // there was no combat
        let mut stats = new_combat_stats(
            game,
            defender,
            m.destination,
            player_index,
            &m.units,
            Some(CombatResult::AttackerWins),
        );

        capture_position(game, &mut stats);

        move_units(
            game,
            player_index,
            &m.units,
            m.destination,
            m.embark_carrier_id,
        );

        end_combat(game, stats);
    } else {
        move_units(
            game,
            player_index,
            &m.units,
            m.destination,
            m.embark_carrier_id,
        );
    }
}
