use crate::content::advances::{NAVIGATION, ROADS};
use crate::game::Game;
use crate::map::Map;
use crate::map::Terrain::{Forest, Mountain};
use crate::payment::PaymentOptions;
use crate::player::Player;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use crate::unit::{MovementRestriction, Unit};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct MoveRoute {
    pub destination: Position,
    pub cost: PaymentOptions,
    pub stack_size_used: usize,
    pub ignore_terrain_movement_restrictions: bool,
}

impl MoveRoute {
    fn simple(destination: Position) -> Self {
        Self {
            destination,
            cost: PaymentOptions::free(),
            stack_size_used: 0,
            ignore_terrain_movement_restrictions: false,
        }
    }
}

pub(crate) fn is_valid_movement_type(
    game: &Game,
    units: &Vec<&Unit>,
    embark_position: Option<Position>,
    dest: Position,
) -> bool {
    if let Some(embark_position) = embark_position {
        return dest == embark_position;
    }
    units.iter().all(|unit| {
        if unit.unit_type.is_land_based() && game.map.is_water(dest) {
            return false;
        }
        if unit.unit_type.is_ship() && game.map.is_land(dest) {
            return false;
        }
        true
    })
}

#[must_use]
pub(crate) fn move_routes(
    starting: Position,
    player: &Player,
    units: &[u32],
    game: &Game,
    embark_carrier_id: Option<u32>,
) -> Vec<MoveRoute> {
    let mut base: Vec<MoveRoute> = starting
        .neighbors()
        .iter()
        .map(|&n| MoveRoute::simple(n))
        .collect();
    if player.has_advance(NAVIGATION) {
        base.extend(reachable_with_navigation(player, units, &game.map));
    }
    if player.has_advance(ROADS) && embark_carrier_id.is_none() {
        base.extend(reachable_with_roads(player, units, game));
    }
    base
}

#[must_use]
fn reachable_with_roads(player: &Player, units: &[u32], game: &Game) -> Vec<MoveRoute> {
    let start = units.iter().find_map(|&id| {
        let unit = player.get_unit(id).expect("unit not found");
        if unit.unit_type.is_land_based() {
            Some(unit.position)
        } else {
            None
        }
    });

    if let Some(start) = start {
        let map = &game.map;
        if map.is_water(start) {
            // not for disembarking
            return vec![];
        };

        return start
            .neighbors()
            .into_iter()
            .flat_map(|middle| {
                // don't move over enemy units or cities
                let stack_size_used = player
                    .get_units(middle)
                    .iter()
                    .filter(|unit| unit.unit_type.is_army_unit())
                    .count();

                if map.is_land(middle) && game.enemy_player(player.index, middle).is_none() {
                    let mut dest: Vec<(Position, usize)> = middle
                        .neighbors()
                        .into_iter()
                        .map(move |n| (n, stack_size_used))
                        .collect();
                    dest.push((middle, stack_size_used));
                    dest.retain(|&(p, _)| p != start);
                    dest
                } else {
                    vec![]
                }
            })
            .into_group_map_by(|&(p, _)| p)
            .into_iter()
            .filter_map(|(destination, stack_sizes_used)| {
                // but can stop on enemy units
                if map.is_land(destination)
                    && (
                        // from or to owned city
                        player.get_city(start).is_some() || player.get_city(destination).is_some()
                    )
                {
                    let stack_size_used =
                        stack_sizes_used.iter().map(|&(_, s)| s).min().expect("min");
                    let route = MoveRoute {
                        destination,
                        cost: PaymentOptions::resources(
                            ResourcePile::ore(1) + ResourcePile::food(1),
                        ),
                        stack_size_used,
                        ignore_terrain_movement_restrictions: true,
                    };
                    Some(route)
                } else {
                    None
                }
            })
            .collect();
    }
    vec![]
}

#[must_use]
fn reachable_with_navigation(player: &Player, units: &[u32], map: &Map) -> Vec<MoveRoute> {
    if !player.has_advance(NAVIGATION) {
        return vec![];
    }
    let ship = units.iter().find_map(|&id| {
        let unit = player.get_unit(id).expect("unit not found");
        if unit.unit_type.is_ship() {
            Some(unit.position)
        } else {
            None
        }
    });
    if let Some(ship) = ship {
        let start = ship.neighbors().into_iter().find(|n| map.is_outside(*n));
        if let Some(start) = start {
            let mut perimeter = vec![ship];

            add_perimeter(map, start, &mut perimeter);
            let can_navigate =
                |p: &Position| *p != ship && (map.is_water(*p) || map.is_unexplored(*p));
            let first = perimeter.iter().copied().find(can_navigate);
            let last = perimeter.iter().copied().rfind(can_navigate);

            return vec![first, last]
                .into_iter()
                .flatten()
                .map(MoveRoute::simple)
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

pub(crate) fn terrain_movement_restriction(
    map: &Map,
    destination: Position,
    unit: &Unit,
) -> Option<MovementRestriction> {
    let terrain = map
        .get(destination)
        .expect("the destination position should exist on the map");
    match terrain {
        Mountain => Some(MovementRestriction::Mountain),
        Forest if unit.unit_type.is_army_unit() => Some(MovementRestriction::Forest),
        _ => None,
    }
}

pub(crate) fn has_movable_units(game: &Game, player: &Player) -> bool {
    player.units.iter().any(|unit| {
        player
            .move_units_destinations(game, &[unit.id], unit.position, None)
            .is_ok()
            || can_embark(game, player, unit)
    })
}

fn can_embark(game: &Game, player: &Player, unit: &Unit) -> bool {
    unit.unit_type.is_land_based()
        && player.units.iter().any(|u| {
            u.unit_type.is_ship()
                && player
                    .move_units_destinations(game, &[unit.id], u.position, Some(u.id))
                    .is_ok()
        })
}
