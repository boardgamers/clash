use crate::advance::Advance;
use crate::city::{City, MoodState};
use crate::events::EventPlayer;
use crate::game::Game;
use crate::payment::ResourceReward;
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::unit::Unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRoute {
    pub(crate) unit_id: u32,
    from: Position,
    pub to: Position,
}

#[must_use]
pub(crate) fn trade_route_reward(
    game: &Game,
    p: &EventPlayer,
) -> Option<(ResourceReward, Vec<TradeRoute>)> {
    let trade_routes = find_trade_routes(game, p.get(game), false);
    if trade_routes.is_empty() {
        return None;
    }

    let reward = p.reward_options().sum(
        trade_routes.len() as u8,
        if p.get(game).can_use_advance(Advance::Currency) {
            &[ResourceType::Gold, ResourceType::Food]
        } else {
            &[ResourceType::Food]
        },
    );
    Some((reward, trade_routes))
}

pub(crate) fn trade_route_log(
    game: &Game,
    player_index: usize,
    trade_routes: &[TradeRoute],
    selected: bool,
) -> Vec<String> {
    let mut log = Vec::new();
    if selected {
        log.push(format!(
            "{} selected trade routes",
            game.player_name(player_index),
        ));
    }
    for t in trade_routes {
        log.push(format!(
            "{} at {} traded with city at {}",
            game.players[player_index]
                .get_unit(t.unit_id)
                .unit_type
                .non_leader_name(),
            t.from,
            t.to,
        ));
    }
    log
}

#[must_use]
pub fn find_trade_routes(game: &Game, player: &Player, only_ships: bool) -> Vec<TradeRoute> {
    let all: Vec<Vec<TradeRoute>> = player
        .units
        .iter()
        .filter(|u| !only_ships || u.is_ship())
        .map(|u| find_trade_route_for_unit(game, player, u))
        .filter(|r| !r.is_empty())
        .collect();
    let mut routes = find_most_trade_routes(&all, 0, &[]);
    routes.truncate(4);
    routes
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

pub(crate) fn find_trade_route_for_unit(
    game: &Game,
    player: &Player,
    unit: &Unit,
) -> Vec<TradeRoute> {
    if !player.can_use_advance(Advance::TradeRoutes) {
        // not only used from the regular Trade Routes method, so we need to check the advance
        return vec![];
    }

    let expected_type = unit.is_ship() || unit.is_settler();
    if !expected_type {
        return vec![];
    }

    game.players
        .iter()
        .filter(|p| p.is_human() && p.index != player.index)
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

    let from = unit.position;
    let distance = from.distance(to.position);
    if distance > 2 {
        return None;
    }

    if game.is_pirate_zone(from) {
        return None;
    }

    let safe_passage = from.neighbors().iter().any(|&pos| {
        pos.neighbors().contains(&to.position)
            && game.map.is_inside(pos)
            && !game.map.is_unexplored(pos)
            && !game.is_pirate_zone(pos)
    });

    if !safe_passage {
        return None;
    }

    Some(TradeRoute {
        unit_id: unit.id,
        from,
        to: to.position,
    })
}
