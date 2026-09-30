use crate::city::activate_city;
use crate::combat;
use crate::construct::NOT_ENOUGH_RESOURCES;
use crate::content::ability::recruit_event_origin;
use crate::content::persistent_events::PersistentEventType;
use crate::game::Game;
use crate::map::capital_city_position;
use crate::payment::PaymentOptions;
use crate::player::{CostTrigger, Player, gain_units};
use crate::player_events::CostInfo;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::unit::{UnitType, Units, kill_units, set_unit_position};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Recruit {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub attack_pirates: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft_card: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ballcourts: bool,
    pub units: Units,
    pub city_position: Position,
    pub payment: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaced_units: Vec<u32>,
}

impl Recruit {
    #[must_use]
    pub fn new(units: &Units, city_position: Position, payment: ResourcePile) -> Self {
        Self {
            ballcourts: false,
            attack_pirates: false,
            draft_card: false,
            units: units.clone(),
            city_position,
            payment,
            replaced_units: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_replaced_units(mut self, replaced_units: &[u32]) -> Self {
        self.replaced_units = replaced_units.to_vec();
        self
    }
}

pub(crate) fn execute_recruit(
    game: &mut Game,
    player_index: usize,
    r: Recruit,
) -> Result<(), String> {
    crate::content::civilizations::carthage::validate_recruit(game, game.player(player_index), &r)?;
    let cost = recruit_cost_with_options(
        game,
        game.player(player_index),
        &r.units,
        r.city_position,
        &r.replaced_units,
        game.execute_cost_trigger(),
        r.ballcourts,
        r.draft_card,
    )?;
    cost.pay(game, &r.payment);
    let origin = cost.origin();
    for unit in &r.replaced_units {
        // kill separately, because they may be on different positions
        kill_units(game, &[*unit], player_index, None, origin);
    }
    let player = game.player_mut(player_index);
    let types = r.units.clone().to_vec();
    player.units.reserve_exact(types.len());
    activate_city(r.city_position, game, origin);
    let mut land = Units::empty();
    let mut ship = Units::empty();

    for unit_type in types {
        if unit_type.is_ship() {
            ship += &unit_type;
        } else {
            land += &unit_type;
        }
    }
    if !land.is_empty() {
        gain_units(game, player_index, r.city_position, land, origin);
    }
    if !ship.is_empty() {
        let position = game
            .player(player_index)
            .get_city(r.city_position)
            .port_position
            .expect("Cannot recruit ships without port");
        gain_units(game, player_index, position, ship, origin);
    }

    if r.attack_pirates {
        let pos = game
            .player(player_index)
            .get_city(r.city_position)
            .port_position
            .unwrap();
        let ids = game
            .player(player_index)
            .get_units(pos)
            .iter()
            .filter(|u| u.is_ship() && !u.pirate)
            .map(|u| u.id)
            .collect();
        let mut movement = crate::movement::MoveUnits::new(ids, pos, None, ResourcePile::empty());
        movement.attack_pirates = true;
        crate::content::civilizations::carthage::prepare_move(game, player_index, &mut movement)?;
    } else {
        crate::content::civilizations::carthage::sync_pirates(game, player_index);
    }
    if r.draft_card {
        game.player_mut(player_index)
            .event_info
            .insert("Shogunate Draft".into(), "used".into());
        crate::action_card::gain_action_card_from_pile(
            game,
            &crate::events::EventPlayer::from_player(
                player_index,
                game,
                crate::events::EventOrigin::SpecialAdvance(SpecialAdvance::Shogunate),
            ),
        );
    }
    on_recruit(game, player_index, r);
    Ok(())
}

pub(crate) fn on_recruit(game: &mut Game, player_index: usize, r: Recruit) {
    let city_position = r.city_position;
    if game
        .trigger_persistent_event(
            &[player_index],
            |events| &mut events.recruit,
            r,
            PersistentEventType::Recruit,
        )
        .is_none()
    {
        return;
    }

    if let Some(port_position) = game.players[player_index]
        .try_get_city(city_position)
        .and_then(|city| city.port_position)
    {
        let ships = game.players[player_index]
            .get_units(port_position)
            .iter()
            .filter(|unit| unit.is_ship())
            .map(|unit| unit.id)
            .collect::<Vec<_>>();
        if !ships.is_empty()
            && let Some(defender) = game.enemy_player(player_index, port_position)
        {
            for ship in game
                .player(player_index)
                .get_units(port_position)
                .iter()
                .map(|unit| unit.id)
                .collect_vec()
            {
                set_unit_position(player_index, ship, city_position, game);
            }
            combat::initiate_combat(game, defender, port_position, player_index, ships, false);
        }
    }
}

///
/// # Errors
///
/// Errors if the cost cannot be paid
pub fn recruit_cost(
    game: &Game,
    player: &Player,
    units: &Units,
    city_position: Position,
    replaced_units: &[u32],
    execute: CostTrigger,
) -> Result<CostInfo, String> {
    recruit_cost_with_ballcourts(
        game,
        player,
        units,
        city_position,
        replaced_units,
        execute,
        false,
    )
}

pub(crate) fn recruit_cost_with_ballcourts(
    game: &Game,
    player: &Player,
    units: &Units,
    city_position: Position,
    replaced_units: &[u32],
    execute: CostTrigger,
    ballcourts: bool,
) -> Result<CostInfo, String> {
    recruit_cost_with_options(
        game,
        player,
        units,
        city_position,
        replaced_units,
        execute,
        ballcourts,
        false,
    )
}
pub(crate) fn recruit_cost_with_options(
    game: &Game,
    player: &Player,
    units: &Units,
    city_position: Position,
    replaced_units: &[u32],
    execute: CostTrigger,
    ballcourts: bool,
    draft_card: bool,
) -> Result<CostInfo, String> {
    let mut require_replace = units.clone();
    for t in player.available_units().to_vec() {
        if require_replace.has_unit(&t) {
            require_replace -= &t;
        }
    }
    let replaced_units = replaced_units
        .iter()
        .map(|id| {
            let unit_type = player.get_unit(*id).unit_type;
            if unit_type.is_leader() {
                require_replace.leader.map_or(unit_type, UnitType::Leader)
            } else {
                unit_type
            }
        })
        .collect();
    if require_replace != replaced_units {
        return Err("Invalid replacement".to_string());
    }
    recruit_cost_options(
        game,
        player,
        units,
        city_position,
        execute,
        ballcourts,
        draft_card,
    )
}

///
/// # Errors
///
/// Errors if the cost cannot be paid
pub fn recruit_cost_without_replaced(
    game: &Game,
    player: &Player,
    units: &Units,
    city_position: Position,
    execute: CostTrigger,
) -> Result<CostInfo, String> {
    recruit_cost_options(game, player, units, city_position, execute, false, false)
}

fn recruit_cost_options(
    game: &Game,
    player: &Player,
    units: &Units,
    city_position: Position,
    execute: CostTrigger,
    ballcourts: bool,
    draft_card: bool,
) -> Result<CostInfo, String> {
    let city = player.get_city(city_position);
    if draft_card
        && (!player.has_special_advance(SpecialAdvance::Shogunate)
            || !player.can_use_advance(crate::advance::Advance::Draft)
            || player.event_info.contains_key("Shogunate Draft"))
    {
        return Err("Shogunate Draft is unavailable".into());
    }
    if units.is_empty() && !draft_card {
        return Err("Choose units or an action card".into());
    }
    if ballcourts
        && !crate::content::civilizations::maya::ballcourts_available(player, city_position)
    {
        return Err("Ballcourts requires a non-Angry city".into());
    }

    let local_elephants = player.has_special_advance(SpecialAdvance::Warbeasts)
        || (player.has_special_advance(SpecialAdvance::IndianElephants)
            && game.map.get(city_position) == Some(&crate::map::Terrain::Forest))
        || (player.has_special_advance(SpecialAdvance::PersianElephants)
            && city_position
                .neighbors()
                .iter()
                .any(|p| game.map.get(*p) == Some(&crate::map::Terrain::Barren)));
    if city.pieces.market.is_none()
        && ((units.elephants > 0 && !local_elephants)
            || (units.cavalry > 0
                && !(is_cavalry_province_city(player, city_position, game)
                    || (player.has_special_advance(SpecialAdvance::Horsemanship)
                        && city.pieces.fortress.is_some())
                    || (player.has_special_advance(SpecialAdvance::MountedArchers)
                        && city.size() == 1))))
    {
        return Err("Missing building: market".into());
    }
    if units.ships > 0 && city.pieces.port.is_none() {
        return Err("Mising building: port".to_string());
    }

    for (t, a) in units.clone() {
        let avail = player.unit_limit().get_amount(&t);
        if a > avail {
            return Err(format!("Only have {avail} {t:?} - not {a}"));
        }
    }
    if !city.can_activate() {
        return Err("City cannot be activated".to_string());
    }
    let mut base_cost = CostInfo::new(
        player,
        PaymentOptions::resources(
            player,
            recruit_event_origin(),
            units.clone().to_vec().iter().map(UnitType::cost).sum(),
        ),
    );
    base_cost.city_position = Some(city_position);
    let mut cost = player.trigger_cost_event(|e| &e.recruit_cost, base_cost, units, game, execute);
    if ballcourts {
        cost.cost.default.mood_tokens += 1;
    }
    if draft_card {
        // The card replaces the Infantry purchased through Draft, so Draft cannot also discount an Infantry.
        cost.cost
            .conversions
            .retain(|c| !c.from.contains(&UnitType::cost(&UnitType::Infantry)));
        cost.cost.default.mood_tokens += crate::content::advances::warfare::draft_cost(player);
    }
    if !player.can_afford(&cost.cost) {
        return Err(NOT_ENOUGH_RESOURCES.to_string());
    }
    if units.amount() + u8::from(draft_card)
        > city.mood_modified_size(player) as u8 + u8::from(ballcourts)
    {
        return Err("Too many units".to_string());
    }
    if player
        .get_units(city_position)
        .iter()
        .filter(|unit| unit.is_army_unit())
        .count() as u8
        + units.amount()
        - units.settlers
        - units.ships
        > if units.leader == Some(crate::leader::Leader::Xerxes) {
            5
        } else {
            crate::content::civilizations::persia::stack_limit(player, city_position) as u8
        }
    {
        return Err("Too many units in stack".to_string());
    }

    if let Some(l) = units.leader
        && !player.available_leaders.contains(&l)
    {
        return Err(format!("Leader {l:?} not available"));
    }

    Ok(cost)
}

fn is_cavalry_province_city(player: &Player, city: Position, game: &Game) -> bool {
    player.has_special_advance(SpecialAdvance::Provinces)
        && capital_city_position(game, player).distance(city) >= 3
}
