use crate::action::Action;
use crate::combat;
use crate::consts::STACK_LIMIT;
use crate::game::Game;
use crate::game::GameState::Playing;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::player_events::CostInfo;
use crate::playing_actions::PlayingAction;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::undo::UndoContext;
use crate::unit::{Unit, UnitType, Units};

pub(crate) fn recruit(
    game: &mut Game,
    player_index: usize,
    units: Units,
    city_position: Position,
    leader_name: Option<&String>,
    replaced_units: &[u32],
) {
    let mut replaced_leader = None;
    if let Some(leader_name) = leader_name {
        if let Some(previous_leader) = game.players[player_index].active_leader.take() {
            Player::with_leader(
                &previous_leader,
                game,
                player_index,
                |game, previous_leader| {
                    (previous_leader.listeners.deinitializer)(game, player_index);
                },
            );
            replaced_leader = Some(previous_leader);
        }
        set_active_leader(game, leader_name.clone(), player_index);
    }
    let mut replaced_units_undo_context = Vec::new();
    for unit in replaced_units {
        let player = game.get_player_mut(player_index);
        let u = player.remove_unit(*unit);
        if u.carrier_id.is_some_and(|c| replaced_units.contains(&c)) {
            // will be removed when the carrier is removed
            continue;
        }
        let unit = u.data(game.get_player(player_index));
        replaced_units_undo_context.push(unit);
    }
    game.push_undo_context(UndoContext::Recruit {
        replaced_units: replaced_units_undo_context,
        replaced_leader,
    });
    let player = game.get_player_mut(player_index);
    let vec = units.to_vec();
    player.units.reserve_exact(vec.len());
    for unit_type in vec {
        let city = player
            .get_city(city_position)
            .expect("player should have a city at the recruitment position");
        let position = match &unit_type {
            UnitType::Ship => city
                .port_position
                .expect("there should be a port in the city"),
            _ => city_position,
        };
        player.add_unit(position, unit_type);
    }
    let city = player
        .get_city_mut(city_position)
        .expect("player should have a city at the recruitment position");
    city.activate();
    on_recruit(game, player_index);
}

fn set_active_leader(game: &mut Game, leader_name: String, player_index: usize) {
    game.players[player_index]
        .available_leaders
        .retain(|name| name != &leader_name);
    Player::with_leader(&leader_name, game, player_index, |game, leader| {
        (leader.listeners.initializer)(game, player_index);
        (leader.listeners.one_time_initializer)(game, player_index);
    });
    game.get_player_mut(player_index).active_leader = Some(leader_name);
}

pub(crate) fn on_recruit(game: &mut Game, player_index: usize) {
    let Some(Action::Playing(PlayingAction::Recruit(r))) = find_last_action(game, |action| {
        matches!(action, Action::Playing(PlayingAction::Recruit(_)))
    }) else {
        panic!("last action should be a recruit action")
    };

    if game.trigger_custom_phase_event(&[player_index], |events| &mut events.on_recruit, &r, None) {
        return;
    }
    let city_position = r.city_position;

    if let Some(port_position) = game.players[player_index]
        .get_city(city_position)
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
                    Some(Playing),
                );
            }
        }
    }
}

fn find_last_action(game: &Game, pred: fn(&Action) -> bool) -> Option<Action> {
    game.action_log
        .iter()
        .rev()
        .find(|item| pred(&item.action))
        .map(|item| item.action.clone())
}

///
///
/// # Panics
///
/// Panics if city does not exist
pub fn undo_recruit(
    game: &mut Game,
    player_index: usize,
    units: Units,
    city_position: Position,
    leader_name: Option<&String>,
) {
    undo_recruit_without_activate(game, player_index, &units.to_vec(), leader_name);
    game.players[player_index]
        .get_city_mut(city_position)
        .expect("player should have a city a recruitment position")
        .undo_activate();
    if let Some(UndoContext::Recruit {
        replaced_units,
        replaced_leader,
    }) = game.pop_undo_context()
    {
        let player = game.get_player_mut(player_index);
        for unit in replaced_units {
            player.units.extend(Unit::from_data(player_index, unit));
        }
        if let Some(replaced_leader) = replaced_leader {
            player.active_leader = Some(replaced_leader.clone());
            Player::with_leader(
                &replaced_leader,
                game,
                player_index,
                |game, replaced_leader| {
                    (replaced_leader.listeners.initializer)(game, player_index);
                    (replaced_leader.listeners.one_time_initializer)(game, player_index);
                },
            );
        }
    }
}

fn undo_recruit_without_activate(
    game: &mut Game,
    player_index: usize,
    units: &[UnitType],
    leader_name: Option<&String>,
) {
    if let Some(leader_name) = leader_name {
        let current_leader = game.players[player_index]
            .active_leader
            .take()
            .expect("the player should have an active leader");
        Player::with_leader(
            &current_leader,
            game,
            player_index,
            |game, current_leader| {
                (current_leader.listeners.deinitializer)(game, player_index);
                (current_leader.listeners.undo_deinitializer)(game, player_index);
            },
        );

        game.players[player_index]
            .available_leaders
            .push(leader_name.clone());
        game.players[player_index].available_leaders.sort();

        game.players[player_index].active_leader = None;
    }
    let player = game.get_player_mut(player_index);
    for _ in 0..units.len() {
        player
            .units
            .pop()
            .expect("the player should have the recruited units when undoing");
        player.next_unit_id -= 1;
    }
}

///
///
/// # Panics
///
/// Panics if city does not exist
#[must_use]
pub fn recruit_cost(
    player: &Player,
    units: &Units,
    city_position: Position,
    leader_name: Option<&String>,
    replaced_units: &[u32],
    execute: Option<&ResourcePile>,
) -> Option<CostInfo> {
    let mut require_replace = units.clone();
    for t in player.available_units().to_vec() {
        let a = require_replace.get_mut(&t);
        if *a > 0 {
            *a -= 1;
        }
    }
    let replaced_units = replaced_units
        .iter()
        .map(|id| {
            player
                .get_unit(*id)
                .expect("player should have units to be replaced")
                .unit_type
        })
        .collect();
    if require_replace != replaced_units {
        return None;
    }
    recruit_cost_without_replaced(player, units, city_position, leader_name, execute)
}

///
///
/// # Panics
///
/// Panics if city does not exist
#[must_use]
pub fn recruit_cost_without_replaced(
    player: &Player,
    units: &Units,
    city_position: Position,
    leader_name: Option<&String>,
    execute: Option<&ResourcePile>,
) -> Option<CostInfo> {
    let city = player
        .get_city(city_position)
        .expect("player should have a city at the recruitment position");
    if !city.can_activate() {
        return None;
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
        return None;
    }
    if vec.len() > city.mood_modified_size(player) {
        return None;
    }
    if vec
        .iter()
        .any(|unit| matches!(unit, UnitType::Cavalry | UnitType::Elephant))
        && city.pieces.market.is_none()
    {
        return None;
    }
    if vec.iter().any(|unit| matches!(unit, UnitType::Ship)) && city.pieces.port.is_none() {
        return None;
    }
    if player
        .get_units(city_position)
        .iter()
        .filter(|unit| unit.unit_type.is_army_unit())
        .count()
        + vec.iter().filter(|unit| unit.is_army_unit()).count()
        > STACK_LIMIT
    {
        return None;
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
        return None;
    }
    Some(cost)
}
