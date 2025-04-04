use crate::action_card::{CivilCardMatch, CivilCardOpportunity};
use crate::city::City;
use crate::city::MoodState::Angry;
use crate::city_pieces::Building;
use crate::combat_listeners::{
    Casualties, CombatEventPhase, CombatRoundEnd, CombatRoundStart, CombatStrength,
    combat_round_end, combat_round_start,
};
use crate::combat_roll::CombatStats;
use crate::consts::SHIP_CAPACITY;
use crate::content::persistent_events::PersistentEventType;
use crate::game::Game;
use crate::movement::{MoveUnits, MovementRestriction, move_units, stop_current_move};
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::tactics_card::CombatRole;
use crate::unit::{UnitType, Units, carried_units};
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
        self.defender_city(game)
            .is_some_and(|city| city.pieces.fortress.is_some())
    }

    #[must_use]
    pub fn defender_city<'a>(&self, game: &'a Game) -> Option<&'a City> {
        game.players[self.defender].try_get_city(self.defender_position)
    }

    #[must_use]
    pub fn defender_temple(&self, game: &Game) -> bool {
        self.defender_city(game)
            .is_some_and(|city| city.pieces.temple.is_some())
    }

    #[must_use]
    pub fn is_sea_battle(&self, game: &Game) -> bool {
        game.map.is_sea(self.defender_position)
    }

    #[must_use]
    pub fn carried_units_casualties(&self, game: &Game, player: usize, casualties: u8) -> u8 {
        if self.is_sea_battle(game) {
            let units = game.player(player).get_units(self.position(player));
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

    #[must_use]
    pub fn players(&self) -> [usize; 2] {
        [self.attacker, self.defender]
    }

    #[must_use]
    pub fn role(&self, player: usize) -> CombatRole {
        if player == self.attacker {
            CombatRole::Attacker
        } else {
            CombatRole::Defender
        }
    }

    #[must_use]
    pub fn player(&self, role: CombatRole) -> usize {
        match role {
            CombatRole::Attacker => self.attacker,
            CombatRole::Defender => self.defender,
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
    log_round(game, &combat);
    start_combat(game, combat);
}

pub(crate) fn log_round(game: &mut Game, c: &Combat) {
    game.add_info_log_group(format!("Combat round {}", c.round));
    game.add_info_log_item(&format!(
        "Attackers: {}",
        c.attackers
            .iter()
            .flat_map(|u| {
                let p = game.player(c.attacker);
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
    ));
    game.add_info_log_item(&format!(
        "Defenders: {}",
        game.player(c.defender)
            .get_units(c.defender_position)
            .iter()
            .map(|u| u.unit_type)
            .collect::<Units>()
    ));
}

pub(crate) fn start_combat(game: &mut Game, combat: Combat) {
    game.lock_undo(); // combat should not be undoable
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

pub(crate) fn update_combat_strength(
    game: &mut Game,
    player: usize,
    e: &mut CombatRoundStart,
    update: impl Fn(&mut Game, &Combat, &mut CombatStrength, CombatRole) + Clone,
) {
    if player == e.combat.attacker {
        update(
            game,
            &e.combat,
            &mut e.attacker_strength,
            CombatRole::Attacker,
        );
    } else if player == e.combat.defender {
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
            };
        }

        let c = s.combat;

        let a_t = s.attacker_strength.tactics_card.take();
        let d_t = s.defender_strength.tactics_card.take();

        let result = if let Some(result) = s.final_result {
            let mut round_end = CombatRoundEnd::new(
                Casualties::new(0, None),
                Casualties::new(0, None),
                false,
                c,
                game,
            );
            round_end.final_result = Some(result);
            round_end.phase = CombatEventPhase::Default;
            round_end
        } else {
            let mut a = CombatStats::roll(c.attacker, &c, game, s.attacker_strength);
            let mut d = CombatStats::roll(c.defender, &c, game, s.defender_strength);

            a.determine_hits(&d, game);
            d.determine_hits(&a, game);

            let can_retreat = matches!(c.retreat, CombatRetreatState::CanRetreat)
                && a.hits < d.fighters
                && d.hits < a.fighters;

            CombatRoundEnd::new(
                Casualties::new(d.hits, a_t),
                Casualties::new(a.hits, d_t),
                can_retreat,
                c,
                game,
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
    new_player_index: usize,
    old_player_index: usize,
) {
    let Some(mut city) = game.players[old_player_index].take_city(position) else {
        panic!("player should have this city")
    };
    game.add_to_last_log_item(&format!(
        " and captured {}'s city at {position}",
        game.player_name(old_player_index)
    ));
    let attacker_is_human = game.player(new_player_index).is_human();
    let size = city.mood_modified_size(&game.players[new_player_index]);
    if attacker_is_human {
        game.players[new_player_index].gain_resources(ResourcePile::gold(size as u32));

        CivilCardMatch::new(CivilCardOpportunity::CaptureCity, Some(old_player_index)).store(game);
    }
    let take_over = game.player(new_player_index).is_city_available();

    if take_over {
        city.player_index = new_player_index;
        city.mood_state = Angry;
        if attacker_is_human {
            for wonder in &city.pieces.wonders {
                wonder.listeners.deinit(game, old_player_index);
                wonder.listeners.init(game, new_player_index);
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
            game.player_name(old_player)
        ));
    }
    for id in captured_settlers {
        game.players[old_player].remove_unit(id);
    }
    if game.player(old_player).try_get_city(position).is_some() {
        conquer_city(game, position, new_player, old_player);
    }
}

fn move_to_enemy_player_tile(
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
            game.player(player_index).is_human(),
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
        if move_to_enemy_player_tile(
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

    use super::{Game, conquer_city};

    use crate::action::Action;
    use crate::game::GameState;
    use crate::log::{ActionLogAge, ActionLogItem, ActionLogPlayer, ActionLogRound};
    use crate::movement::MovementAction;
    use crate::payment::PaymentOptions;
    use crate::utils::tests::FloatEq;
    use crate::wonder::construct_wonder;
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
        let mut age = ActionLogAge::new();
        let mut round = ActionLogRound::new();
        let mut log = ActionLogPlayer::new(0);
        log.items
            .push(ActionLogItem::new(Action::Movement(MovementAction::Stop)));
        round.players.push(log);
        age.rounds.push(round);
        Game {
            state: GameState::Playing,
            events: Vec::new(),
            players: Vec::new(),
            map: Map::new(HashMap::new()),
            starting_player_index: 0,
            current_player_index: 0,
            action_log: vec![age],
            action_log_index: 0,
            current_action_log_index: None,
            log: Vec::new(),
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
            action_cards_left: Vec::new(),
            incidents_left: Vec::new(),
            permanent_effects: Vec::new(),
        }
    }

    #[test]
    fn conquer_test() {
        let old = Player::new(civilizations::tests::get_test_civilization(), 0);
        let new = Player::new(civilizations::tests::get_test_civilization(), 1);

        let wonder = Wonder::builder("wonder", "test", PaymentOptions::free(), vec![]).build();
        let mut game = test_game();
        game.add_info_log_group("combat".into()); // usually filled in combat
        game.players.push(old);
        game.players.push(new);
        let old = 0;
        let new = 1;

        let position = Position::new(0, 0);
        game.players[old].cities.push(City::new(old, position));
        construct_wonder(&mut game, wonder, position, old);
        game.players[old].construct(Academy, position, None, true);
        game.players[old].construct(Obelisk, position, None, true);

        game.players[old].victory_points(&game).assert_eq(7.0);

        conquer_city(&mut game, position, new, old);

        let c = game.players[new].get_city_mut(position);
        assert_eq!(1, c.player_index);
        assert_eq!(Angry, c.mood_state);

        let old = &game.players[old];
        let new = &game.players[new];
        old.victory_points(&game).assert_eq(3.0);
        new.victory_points(&game).assert_eq(4.0);
        assert_eq!(0, old.wonders_owned());
        assert_eq!(1, new.wonders_owned());
        assert_eq!(1, old.owned_buildings(&game));
        assert_eq!(1, new.owned_buildings(&game));
    }
}
