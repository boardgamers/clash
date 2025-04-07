use crate::combat;
use crate::consts::STACK_LIMIT;
use crate::content::persistent_events::PersistentEventType;
use crate::game::Game;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::CostInfo;
use crate::playing_actions::Recruit;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::{UnitType, Units, kill_units};

pub(crate) fn recruit(game: &mut Game, player_index: usize, r: Recruit) -> Result<(), String> {
    let cost = recruit_cost(
        game.player(player_index),
        &r.units,
        r.city_position,
        r.leader_name.as_ref(),
        &r.replaced_units,
        Some(&r.payment),
    )?;
    cost.pay(game, &r.payment);
    for unit in &r.replaced_units {
        // kill separately, because they may be on different positions
        kill_units(game, &[*unit], player_index, None);
    }
    if let Some(leader_name) = &r.leader_name {
        if let Some(previous_leader) = game.player_mut(player_index).active_leader.take() {
            Player::with_leader(
                &previous_leader,
                game,
                player_index,
                |game, previous_leader| {
                    previous_leader.listeners.deinit(game, player_index);
                },
            );
        }
        set_active_leader(game, leader_name.clone(), player_index);
    }
    let player = game.player_mut(player_index);
    let vec = r.units.clone().to_vec();
    player.units.reserve_exact(vec.len());
    for unit_type in vec {
        let city = player.get_city(r.city_position);
        let position = match &unit_type {
            UnitType::Ship => city
                .port_position
                .expect("there should be a port in the city"),
            _ => r.city_position,
        };
        player.add_unit(position, unit_type);
    }
    let city = player.get_city_mut(r.city_position);
    city.activate();
    on_recruit(game, player_index, r);
    Ok(())
}

fn set_active_leader(game: &mut Game, leader_name: String, player_index: usize) {
    game.players[player_index]
        .available_leaders
        .retain(|name| name != &leader_name);
    Player::with_leader(&leader_name, game, player_index, |game, leader| {
        leader.listeners.one_time_init(game, player_index);
    });
    game.player_mut(player_index).active_leader = Some(leader_name);
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
            .filter(|unit| unit.unit_type.is_ship())
            .map(|unit| unit.id)
            .collect::<Vec<_>>();
        if !ships.is_empty() {
            if let Some(defender) = game.enemy_player(player_index, port_position) {
                for ship in game.players[player_index].get_units_mut(port_position) {
                    ship.position = city_position;
                }
                combat::initiate_combat(
                    game,
                    defender,
                    port_position,
                    player_index,
                    city_position,
                    ships,
                    false,
                );
            }
        }
    }
}

///
/// # Errors
///
/// Errors if the cost cannot be paid
pub fn recruit_cost(
    player: &Player,
    units: &Units,
    city_position: Position,
    leader_name: Option<&String>,
    replaced_units: &[u32],
    execute: Option<&ResourcePile>,
) -> Result<CostInfo, String> {
    let mut require_replace = units.clone();
    for t in player.available_units().to_vec() {
        let a = require_replace.get_mut(&t);
        if *a > 0 {
            *a -= 1;
        }
    }
    let replaced_units = replaced_units
        .iter()
        .map(|id| player.get_unit(*id).unit_type)
        .collect();
    if require_replace != replaced_units {
        return Err("Invalid replacement".to_string());
    }
    recruit_cost_without_replaced(player, units, city_position, leader_name, execute)
}

///
/// # Errors
///
/// Errors if the cost cannot be paid
pub fn recruit_cost_without_replaced(
    player: &Player,
    units: &Units,
    city_position: Position,
    leader_name: Option<&String>,
    execute: Option<&ResourcePile>,
) -> Result<CostInfo, String> {
    let city = player.get_city(city_position);
    if !city.can_activate() {
        return Err("City cannot be activated".to_string());
    }
    let vec = units.clone().to_vec();
    let cost = player.trigger_cost_event(
        |e| &e.recruit_cost,
        &PaymentOptions::resources(vec.iter().map(UnitType::cost).sum()),
        units,
        player,
        execute,
    );
    if !player.can_afford(&cost.cost) {
        return Err("Cannot afford".to_string());
    }
    if vec.len() > city.mood_modified_size(player) {
        return Err("Too many units".to_string());
    }
    if vec
        .iter()
        .any(|unit| matches!(unit, UnitType::Cavalry | UnitType::Elephant))
        && city.pieces.market.is_none()
    {
        return Err("No market".to_string());
    }
    if vec.iter().any(|unit| matches!(unit, UnitType::Ship)) && city.pieces.port.is_none() {
        return Err("No port".to_string());
    }
    if player
        .get_units(city_position)
        .iter()
        .filter(|unit| unit.unit_type.is_army_unit())
        .count()
        + vec.iter().filter(|unit| unit.is_army_unit()).count()
        > STACK_LIMIT
    {
        return Err("Too many units in stack".to_string());
    }

    let leaders = vec
        .iter()
        .filter(|unit| matches!(unit, UnitType::Leader))
        .count();
    let match_leader = match leaders {
        0 => leader_name.is_none(),
        1 => leader_name.is_some_and(|n| player.available_leaders.contains(n)),
        _ => false,
    };
    if !match_leader {
        return Err("Invalid leader".to_string());
    }
    Ok(cost)
}
