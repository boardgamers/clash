use crate::advance::Advance;
use crate::consts::STACK_LIMIT;
use crate::content::action_cards::negotiation::negotiations_partner;
use crate::content::incidents::great_diplomat::{DIPLOMAT_ID, diplomatic_relations_partner};
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Map;
use crate::movement::move_event_origin;
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::special_advance::SpecialAdvance;
use itertools::Itertools;
use pathfinding::prelude::astar;

#[derive(Debug, Clone)]
pub struct MoveRoute {
    pub destination: Position,
    pub cost: PaymentOptions,
    pub ignore_terrain_movement_restrictions: bool,
}

impl MoveRoute {
    fn new(
        destination: Position,
        player: &Player,
        cost: ResourcePile,
        modifiers: Vec<EventOrigin>,
    ) -> Self {
        let mut options = PaymentOptions::resources(player, move_event_origin(), cost);
        options.modifiers = modifiers;
        Self {
            destination,
            cost: options,
            ignore_terrain_movement_restrictions: false,
        }
    }
}

#[must_use]
pub(crate) fn move_routes(
    starting: Position,
    player: &Player,
    units: &[u32],
    game: &Game,
    embark_carrier_id: Option<u32>,
    stack_size: usize,
) -> Vec<MoveRoute> {
    let mut base: Vec<MoveRoute> = starting
        .neighbors()
        .iter()
        .filter(|&n| game.map.is_inside(*n))
        .map(|&n| MoveRoute::new(n, player, ResourcePile::empty(), vec![]))
        .collect();
    if player.can_use_advance(Advance::Navigation) {
        base.extend(reachable_with_navigation(player, units, &game.map));
    }
    if player.can_use_advance(Advance::Roads) && embark_carrier_id.is_none() {
        base.extend(reachable_with_roads(player, units, game, stack_size));
    }
    add_diplomatic_relations(player, game, &mut base);
    add_negotiations(player, game, &mut base);
    base
}

fn add_diplomatic_relations(player: &Player, game: &Game, base: &mut Vec<MoveRoute>) {
    if let Some(partner) = diplomatic_relations_partner(game, player.index) {
        let partner = game.player(partner);
        for r in base {
            if !partner.get_units(r.destination).is_empty() {
                r.cost.default += ResourcePile::culture_tokens(2);
                r.cost.modifiers.push(EventOrigin::Incident(DIPLOMAT_ID));
            }
        }
    }
}

fn add_negotiations(player: &Player, game: &Game, base: &mut Vec<MoveRoute>) {
    if let Some(partner) = negotiations_partner(game, player.index) {
        let partner = game.player(partner);
        base.retain(|r| partner.get_units(r.destination).is_empty());
    }
}

#[must_use]
fn reachable_with_roads(
    player: &Player,
    units: &[u32],
    game: &Game,
    stack_size: usize,
) -> Vec<MoveRoute> {
    let start = units.iter().find_map(|&id| {
        let unit = player.get_unit(id);
        if unit.is_land_based() {
            Some(unit.position)
        } else {
            None
        }
    });

    if let Some(start) = start {
        let map = &game.map;
        if map.is_sea(start) {
            // not for disembarking
            return vec![];
        }

        let roman_roads = player.has_special_advance(SpecialAdvance::RomanRoads);
        let mut routes: Vec<MoveRoute> = next_road_step(player, game, start, stack_size)
            .into_iter()
            .flat_map(|middle| next_road_step(player, game, middle, stack_size))
            .unique()
            .filter_map(|destination| {
                road_route(
                    player,
                    start,
                    destination,
                    roman_roads,
                    vec![EventOrigin::Advance(Advance::Roads)],
                )
            })
            .collect();

        if roman_roads {
            routes.extend(roman_roads_routes(player, game, start, stack_size));
        }

        return routes;
    }
    vec![]
}

const ROMAN_ROADS_LENGTH: u8 = 4;

fn roman_roads_routes(
    player: &Player,
    game: &Game,
    start: Position,
    stack_size: usize,
) -> Vec<MoveRoute> {
    if game.try_get_any_city(start).is_none() {
        return vec![];
    }

    player
        .cities
        .iter()
        .filter_map(|city| {
            let distance = city.position.distance(start) as u8;
            if distance > ROMAN_ROADS_LENGTH {
                return None;
            }
            let dst = city.position;

            let len = astar(
                &start,
                |p| {
                    next_road_step(player, game, *p, stack_size)
                        .iter()
                        .map(|&n| (n, 1))
                        .collect_vec()
                },
                |p| p.distance(dst),
                |&p| p == dst,
            )
            .map_or(u8::MAX, |(_path, len)| len as u8);
            if len > ROMAN_ROADS_LENGTH {
                return None;
            }
            road_route(
                player,
                start,
                dst,
                false,
                vec![EventOrigin::SpecialAdvance(SpecialAdvance::RomanRoads)],
            )
        })
        .collect()
}

fn road_route(
    player: &Player,
    start: Position,
    destination: Position,
    ignore_city_to_city: bool,
    modifiers: Vec<EventOrigin>,
) -> Option<MoveRoute> {
    if destination.distance(start) <= 1 {
        // can go directly without using roads
        return None;
    }

    // but can stop on enemy units

    let from_city = player.try_get_city(start).is_some();
    let to_city = player.try_get_city(destination).is_some();
    if !from_city && !to_city {
        return None;
    }
    if from_city && to_city && ignore_city_to_city {
        return None;
    }

    let mut route = MoveRoute::new(
        destination,
        player,
        ResourcePile::ore(1) + ResourcePile::food(1),
        modifiers,
    );
    route.ignore_terrain_movement_restrictions = true;
    Some(route)
}

fn next_road_step(
    player: &Player,
    game: &Game,
    from: Position,
    stack_size: usize,
) -> Vec<Position> {
    // don't move over enemy units or cities
    from.neighbors()
        .into_iter()
        .filter(|to| {
            let on_target = player
                .get_units(*to)
                .iter()
                .filter(|unit| unit.is_army_unit())
                .count();
            game.map.is_land(*to)
                && game.enemy_player(player.index, *to).is_none()
                && on_target + stack_size <= STACK_LIMIT
        })
        .collect_vec()
}

#[must_use]
fn reachable_with_navigation(player: &Player, units: &[u32], map: &Map) -> Vec<MoveRoute> {
    let ship = units.iter().find_map(|&id| {
        let unit = player.get_unit(id);
        if unit.is_ship() {
            Some(unit.position)
        } else {
            None
        }
    });
    if let Some(ship) = ship {
        let start = ship.neighbors().into_iter().find(|n| {
            // otherwise we might have the ship position as only neighbor
            map.is_outside(*n)
                && n.neighbors()
                    .iter()
                    .filter(|nn| map.is_inside(**nn))
                    .count()
                    > 1
        });
        if let Some(start) = start {
            let mut perimeter = vec![ship];

            add_perimeter(map, start, &mut perimeter);
            let can_navigate =
                |p: &Position| *p != ship && (map.is_sea(*p) || map.is_unexplored(*p));
            let first = perimeter.iter().copied().find(can_navigate);
            let last = perimeter.iter().copied().rfind(can_navigate);

            return vec![first, last]
                .into_iter()
                .flatten()
                .map(|destination| {
                    MoveRoute::new(
                        destination,
                        player,
                        ResourcePile::empty(),
                        vec![EventOrigin::Advance(Advance::Navigation)],
                    )
                })
                .collect();
        }
    }
    vec![]
}

fn add_perimeter(map: &Map, start: Position, perimeter: &mut Vec<Position>) {
    if perimeter.contains(&start) {
        return;
    }
    perimeter.push(start);

    let option = &start
        .neighbors()
        .into_iter()
        .filter(|n| {
            !perimeter.contains(n)
                && (map.is_inside(*n) && n.neighbors().iter().any(|n| map.is_outside(*n)))
        })
        // take with most outside neighbors first
        .sorted_by_key(|n| n.neighbors().iter().filter(|n| map.is_inside(**n)).count())
        .next();

    if let Some(n) = option {
        add_perimeter(map, *n, perimeter);
    }
}
