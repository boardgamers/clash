use crate::city::{City, MoodState};
use crate::content::advances::CURRENCY;
use crate::content::custom_phase_actions::CustomPhaseState;
use crate::game::Game;
use crate::game::GameState::CustomPhase;
use crate::payment::PaymentModel;
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::Unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRoute {
    unit_id: u32,
    from: Position,
    to: Position,
}

#[must_use] 
pub fn trade_route_reward(game: &Game) -> Option<(PaymentModel, Vec<TradeRoute>)> {
    let p = game.current_player_index;
    let trade_routes = find_trade_routes(game, &game.players[p]);
    if trade_routes.is_empty() {
        return None;
    }

    Some((
        if game.players[p].has_advance(CURRENCY) {
            PaymentModel::sum(
                trade_routes.len() as u32,
                &[ResourceType::Gold, ResourceType::Food],
            )
        } else {
            PaymentModel::sum(trade_routes.len() as u32, &[ResourceType::Food])
        },
        trade_routes,
    ))
}

pub(crate) fn collect_trade_routes_for_current_player(game: &mut Game) {
    let r = trade_route_reward(game);

    let Some((reward, routes)) = r else {
        return;
    };

    if reward.possible_resource_types().len() > 1 {
        game.state = CustomPhase(CustomPhaseState::TradeRouteSelection);
        return;
    }

    gain_trade_route_reward(
        game,
        game.current_player_index,
        routes.as_slice(),
        &reward.default_payment(),
    );
}

pub(crate) fn gain_trade_route_reward(
    game: &mut Game,
    player_index: usize,
    trade_routes: &[TradeRoute],
    reward: &ResourcePile,
) {
    for t in trade_routes {
        game.add_to_last_log_item(&format!(
            ". {:?} at {:?} traded with city at {:?}",
            game.players[player_index]
                .get_unit(t.unit_id)
                .expect("unit should exist")
                .unit_type,
            t.from,
            t.to,
        ));
    }
    game.add_to_last_log_item(&format!(". Total reward is {reward}"));
    game.players[player_index].gain_resources(reward.clone());
}

#[must_use]
pub fn find_trade_routes(game: &Game, player: &Player) -> Vec<TradeRoute> {
    let all: Vec<Vec<TradeRoute>> = player
        .units
        .iter()
        .map(|u| find_trade_route_for_unit(game, player, u))
        .filter(|r| !r.is_empty())
        .collect();
    find_most_trade_routes(&all, 0, &[])
}

fn find_most_trade_routes(
    all: &[Vec<TradeRoute>],
    unit_index: usize,
    used_cities: &[Position],
) -> Vec<TradeRoute> {
    if unit_index == all.len() {
        return vec![];
    }
    let unit_routes: Vec<TradeRoute> = all[unit_index]
        .iter()
        .filter(|&&r| !used_cities.contains(&r.to))
        .copied()
        .collect();
    unit_routes
        .iter()
        .map(|r| {
            let mut new_used_cities = used_cities.to_vec();
            new_used_cities.push(r.to);
            let mut new_all = all.to_vec();
            new_all[unit_index] = vec![*r];
            let mut new_routes = find_most_trade_routes(&new_all, unit_index + 1, &new_used_cities);
            new_routes.push(*r);
            new_routes
        })
        .max_by_key(Vec::len)
        .unwrap_or_else(Vec::new)
}

fn find_trade_route_for_unit(game: &Game, player: &Player, unit: &Unit) -> Vec<TradeRoute> {
    let expected_type = unit.unit_type.is_ship() || unit.unit_type.is_settler();
    if !expected_type {
        return vec![];
    }

    game.players
        .iter()
        .filter(|p| p.index != player.index)
        .flat_map(|p| p.cities.iter())
        .filter_map(|c| find_trade_route_to_city(game, player, unit, c))
        .collect()
}

fn find_trade_route_to_city(
    game: &Game,
    player: &Player,
    unit: &Unit,
    to: &City,
) -> Option<TradeRoute> {
    if to.player_index == player.index {
        return None;
    }

    if to.mood_state == MoodState::Angry {
        return None;
    }

    let distance = unit.position.distance(to.position);
    if distance > 2 {
        return None;
    }

    let safe_passage = unit
        .position
        .neighbors()
        .iter()
        .any(|&pos| {
            pos.neighbors().contains(&to.position)
                && game.map.is_inside(pos)
                && !game.map.is_unexplored(pos)
        });

    if !safe_passage {
        return None;
    }

    Some(TradeRoute {
        unit_id: unit.id,
        from: unit.position,
        to: to.position,
    })
}
