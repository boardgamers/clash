use crate::advance::Advance;
use crate::city::{City, MoodState};
use crate::content::civilizations::phoenicia::leader_at;
use crate::events::EventPlayer;
use crate::game::Game;
use crate::leader::Leader;
use crate::payment::{PaymentConversion, ResourceReward};
use crate::player::Player;
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use crate::unit::Unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRoute {
    pub(crate) unit_id: Option<u32>,
    pub(crate) from: Position,
    pub to: Position,
}

#[must_use]
pub(crate) fn trade_route_reward(
    game: &Game,
    p: &EventPlayer,
) -> Option<(ResourceReward, Vec<TradeRoute>)> {
    trade_route_reward_with_currency(game, p, p.get(game).can_use_advance(Advance::Currency))
}

// A borrowed Great Library effect expires at the end of this turn. Forecasts
// share the real route and reward rules, but must not rely on borrowed advances.
pub(crate) fn next_turn_trade_route_reward(
    game: &Game,
    p: &EventPlayer,
) -> Option<(ResourceReward, Vec<TradeRoute>)> {
    let permanent = |advance| {
        p.get(game).has_advance(advance)
            || crate::content::civilizations::egypt::grants_advance(p.get(game), advance)
    };
    if !permanent(Advance::TradeRoutes) {
        return None;
    }
    trade_route_reward_with_currency(game, p, permanent(Advance::Currency))
}

fn trade_route_reward_with_currency(
    game: &Game,
    p: &EventPlayer,
    currency: bool,
) -> Option<(ResourceReward, Vec<TradeRoute>)> {
    let trade_routes = find_trade_routes(game, p.get(game), false);
    if trade_routes.is_empty() {
        return None;
    }

    let mut reward = p.reward_options().sum(
        trade_routes.len() as u8,
        if currency {
            if p.get(game).has_special_advance(SpecialAdvance::Alphabet) {
                &[ResourceType::Food, ResourceType::Gold]
            } else {
                &[ResourceType::Gold, ResourceType::Food]
            }
        } else {
            &[ResourceType::Food]
        },
    );
    if p.get(game).has_special_advance(SpecialAdvance::Prosperity) {
        reward = p
            .reward_options()
            .sum(trade_routes.len() as u8, &[ResourceType::Food]);
        for resource in [ResourceType::MoodTokens, ResourceType::Gold] {
            if resource != ResourceType::Gold || currency {
                reward
                    .payment_options
                    .conversions
                    .push(PaymentConversion::unlimited(
                        ResourcePile::food(1),
                        ResourcePile::of(resource, 1),
                    ));
            }
        }
        let influenced = trade_routes
            .iter()
            .filter(|r| {
                !game
                    .get_any_city(r.to)
                    .pieces
                    .buildings(Some(p.index))
                    .is_empty()
            })
            .count() as u8;
        if influenced > 0 {
            reward
                .payment_options
                .conversions
                .push(PaymentConversion::limited(
                    ResourcePile::food(1),
                    ResourcePile::culture_tokens(1),
                    influenced,
                ));
        }
    }
    if p.get(game).has_special_advance(SpecialAdvance::Alphabet) {
        // Route -> culture -> two ideas shares one two-route limit across both choices.
        reward
            .payment_options
            .conversions
            .push(PaymentConversion::limited(
                ResourcePile::food(1),
                ResourcePile::culture_tokens(1),
                2,
            ));
        reward
            .payment_options
            .conversions
            .push(PaymentConversion::unlimited(
                ResourcePile::culture_tokens(1),
                ResourcePile::ideas(2),
            ));
    }
    if p.get(game).has_special_advance(SpecialAdvance::TribalTrade) {
        let human = trade_routes
            .iter()
            .filter(|r| game.player(game.get_any_city(r.to).player_index).is_human())
            .count() as u8;
        reward = p
            .reward_options()
            .sum(trade_routes.len() as u8, &[ResourceType::Food]);
        if currency && human > 0 {
            reward
                .payment_options
                .conversions
                .push(PaymentConversion::limited(
                    ResourcePile::food(1),
                    ResourcePile::gold(1),
                    human,
                ));
        }
    }
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
            "{} at {} traded with city {}",
            t.unit_id
                .map_or("Port city", |id| game.players[player_index]
                    .get_unit(id)
                    .unit_type
                    .non_leader_name()),
            t.from,
            t.to,
        ));
    }
    log
}

#[must_use]
pub fn find_trade_routes(game: &Game, player: &Player, only_ships: bool) -> Vec<TradeRoute> {
    let mut all: Vec<Vec<TradeRoute>> = player
        .units
        .iter()
        .filter(|u| !only_ships || u.is_ship())
        .map(|u| find_trade_route_for_unit(game, player, u))
        .filter(|r| !r.is_empty())
        .collect();
    if !only_ships && player.has_special_advance(SpecialAdvance::CedarsAndDyes) {
        for city in &player.cities {
            if city.pieces.port.is_some() {
                let routes = target_cities(game, player)
                    .filter_map(|to| {
                        find_trade_route_to_city(game, player, None, city.position, true, to)
                    })
                    .collect::<Vec<_>>();
                if !routes.is_empty() {
                    all.push(routes);
                }
            }
        }
    }
    let free_routes = player
        .units
        .iter()
        .filter(|u| (u.is_settler() || u.is_ship()) && leader_at(player, Leader::Hiram, u.position))
        .map(|u| u.id)
        .collect::<Vec<_>>();
    let preferred = if player.has_special_advance(SpecialAdvance::Prosperity) {
        game.players
            .iter()
            .flat_map(|p| &p.cities)
            .filter(|c| !c.pieces.buildings(Some(player.index)).is_empty())
            .map(|c| c.position)
            .collect()
    } else {
        vec![]
    };
    best_routes(&all, 0, &[], &preferred, &free_routes, false)
}

#[cfg(test)]
fn find_most_trade_routes(
    all: &[Vec<TradeRoute>],
    index: usize,
    used: &[Position],
    preferred: &[Position],
) -> Vec<TradeRoute> {
    best_routes(all, index, used, preferred, &[], false)
}

fn best_routes(
    all: &[Vec<TradeRoute>],
    index: usize,
    used: &[Position],
    preferred: &[Position],
    free_sources: &[u32],
    used_free: bool,
) -> Vec<TradeRoute> {
    if index == all.len() || used.len() == 5 {
        return vec![];
    }
    let mut options = vec![best_routes(
        all,
        index + 1,
        used,
        preferred,
        free_sources,
        used_free,
    )];
    for route in &all[index] {
        let free = used_free || route.unit_id.is_some_and(|id| free_sources.contains(&id));
        if used.contains(&route.to) || used.len() >= 4 + usize::from(free) {
            continue;
        }
        let mut next_used = used.to_vec();
        next_used.push(route.to);
        let mut routes = best_routes(all, index + 1, &next_used, preferred, free_sources, free);
        routes.push(*route);
        options.push(routes);
    }
    options
        .into_iter()
        .max_by_key(|routes| {
            (
                routes.len(),
                routes.iter().filter(|r| preferred.contains(&r.to)).count(),
            )
        })
        .unwrap_or_default()
}

fn target_cities<'a>(game: &'a Game, player: &Player) -> impl Iterator<Item = &'a City> {
    let index = player.index;
    let tribal = player.has_special_advance(SpecialAdvance::TribalTrade);
    game.players
        .iter()
        .filter(move |p| {
            (p.is_human() || (tribal && p.civilization.is_barbarian())) && p.index != index
        })
        .flat_map(|p| &p.cities)
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

    let expected_type = unit.is_ship()
        || unit.is_settler()
        || (unit.unit_type == crate::unit::UnitType::Elephant
            && player.has_special_advance(SpecialAdvance::IndianElephants)
            && player
                .try_get_city(unit.position)
                .is_some_and(|c| c.mood_state != MoodState::Angry));
    if !expected_type || crate::content::civilizations::huns::raided_settler(game, unit) {
        return vec![];
    }

    target_cities(game, player)
        .filter_map(|c| {
            find_trade_route_to_city(
                game,
                player,
                Some(unit.id),
                unit.position,
                unit.is_ship(),
                c,
            )
        })
        .collect()
}

fn find_trade_route_to_city(
    game: &Game,
    player: &Player,
    unit_id: Option<u32>,
    from: Position,
    maritime: bool,
    to: &City,
) -> Option<TradeRoute> {
    if to.player_index == player.index {
        return None;
    }

    if to.mood_state == MoodState::Angry {
        return None;
    }

    let extended = maritime
        && player.has_special_advance(SpecialAdvance::CedarsAndDyes)
        && to
            .position
            .neighbors()
            .iter()
            .any(|pos| game.map.is_sea(*pos));
    let range = if extended { 3 } else { 2 };
    if unit_id.is_none() && !extended {
        return None;
    }
    let distance = from.distance(to.position);
    if distance > range {
        return None;
    }

    if game.is_pirate_zone(from) {
        return None;
    }

    let mut frontier = vec![from];
    let mut seen = std::collections::HashSet::from([from]);
    let mut safe_passage = false;
    for _ in 0..range {
        let mut next = vec![];
        for position in frontier {
            for pos in position.neighbors() {
                if pos == to.position {
                    safe_passage = true;
                }
                if game.map.is_inside(pos)
                    && !game.map.is_unexplored(pos)
                    && !game.is_pirate_zone(pos)
                    && seen.insert(pos)
                {
                    next.push(pos);
                }
            }
        }
        frontier = next;
    }

    if !safe_passage {
        return None;
    }

    Some(TradeRoute {
        unit_id,
        from,
        to: to.position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(unit_id: u32, target: &str) -> TradeRoute {
        TradeRoute {
            unit_id: Some(unit_id),
            from: Position::from_offset("A1"),
            to: Position::from_offset(target),
        }
    }

    #[test]
    fn a_unit_without_an_unused_target_does_not_hide_later_routes() {
        let all = vec![
            vec![route(0, "A2")],
            vec![route(1, "A2")],
            vec![route(2, "B2")],
        ];
        let routes = find_most_trade_routes(&all, 0, &[], &[]);
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().any(|r| r.to == Position::from_offset("B2")));
    }

    #[test]
    fn prosperity_keeps_influenced_targets_within_the_four_route_limit() {
        let all = ["A2", "A3", "B2", "B3", "C2"]
            .iter()
            .enumerate()
            .map(|(id, target)| vec![route(id as u32, target)])
            .collect::<Vec<_>>();
        let preferred = Position::from_offset("C2");
        let routes = find_most_trade_routes(&all, 0, &[], &[preferred]);
        assert_eq!(routes.len(), 4);
        assert!(routes.iter().any(|r| r.to == preferred));
    }
}
