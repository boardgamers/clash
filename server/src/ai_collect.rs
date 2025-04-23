use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city::City;
use crate::collect::{
    CollectInfo, PositionCollection, add_collect, apply_total_collect,
    possible_resource_collections, tiles_used,
};
use crate::content::builtin::Builtin;
use crate::game::Game;
use crate::player::{CostTrigger, Player};
use crate::playing_actions::{Collect, PlayingActionType};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;

pub fn set_city_collections(game: &mut Game, city_position: Position) {
    let city = game.get_any_city(city_position);
    let player = city.player_index;
    let p = city_collections_uncached(game, game.player(player), city);
    game.player_mut(player)
        .get_city_mut(city_position)
        .possible_collections
        .clone_from(&p);
}

#[must_use]
pub fn city_collections_uncached(game: &Game, player: &Player, city: &City) -> Vec<Collect> {
    let info = possible_resource_collections(
        game,
        city.position,
        player.index,
        &[],
        CostTrigger::NoModifiers,
    );

    let all = ResourceType::all()
        .into_iter()
        .filter(|r| {
            info.choices
                .iter()
                .any(|(_, choices)| choices.iter().any(|pile| pile.get(r) > 0))
        })
        .collect_vec();
    let l = all.len();
    all.into_iter()
        .permutations(l)
        .filter_map(|priority| city_collection_uncached(game, player, city, &priority))
        .unique_by(|c| c.total.clone())
        .collect_vec()
}

fn city_collection_uncached(
    game: &Game,
    player: &Player,
    city: &City,
    priority: &[ResourceType],
) -> Option<Collect> {
    let mut c: Vec<PositionCollection> = vec![];

    loop {
        let info = possible_resource_collections(
            game,
            city.position,
            player.index,
            &c,
            CostTrigger::NoModifiers,
        );

        let Some((pos, pile)) = pick_resource(&info, &c, priority) else {
            return apply_total_collect(&c, player, info, game)
                .ok()
                .map(|i| Collect::new(city.position, c, i.total, PlayingActionType::Collect));
        };
        c = add_collect(&info, pos, &pile, &c);
    }
}

fn pick_resource(
    info: &CollectInfo,
    collected: &[PositionCollection],
    priority: &[ResourceType],
) -> Option<(Position, ResourcePile)> {
    if info.max_selection == tiles_used(collected) {
        return None;
    }

    let used = collected
        .iter()
        .chunk_by(|c| c.position)
        .into_iter()
        .map(|(p, group)| (p, group.map(|c| c.times).sum::<u8>()))
        .collect_vec();

    let available = info
        .choices
        .iter()
        // .sorted_by_key(|(pos, _)| **pos)
        .filter(|(pos, _)| {
            let u = used
                .iter()
                .find_map(|(p, u)| (*p == **pos).then_some(*u))
                .unwrap_or(0);

            u < info.max_per_tile
        })
        .collect_vec();

    priority.iter().find_map(|r| {
        available.iter().find_map(|(pos, choices)| {
            choices
                .iter()
                .find_map(|pile| (pile.get(r) > 0).then_some((**pos, pile.clone())))
        })
    })
}

pub(crate) fn invalidate_collect_cache() -> Builtin {
    Builtin::builder("InvalidateCollectCache", "-")
        .add_simple_persistent_event_listener(
            |event| &mut event.found_city,
            1,
            |game, player, _, p| {
                reset_collect_within_range(player, *p, game);
            },
        )
        .add_simple_persistent_event_listener(
            |event| &mut event.combat_end,
            0,
            |game, _player, _, p| {
                reset_collect_within_range_for_all(game, p.combat.defender_position);
            },
        )
        .build()
}

pub(crate) fn reset_collect_within_range(player: usize, position: Position, game: &mut Game) {
    let is_land = game.map.is_land(position);
    let p = game.player_mut(player);
    let range = if is_land && p.has_advance(Advance::Husbandry) {
        2
    } else {
        1
    };
    for c in &mut p.cities {
        if c.position.distance(position) <= range {
            c.possible_collections.clear();
        }
    }
}

pub(crate) fn reset_collect_within_range_for_all(game: &mut Game, pos: Position) {
    for p in 0..game.human_players_count() {
        reset_collect_within_range(p, pos, game);
    }
}

pub(crate) fn reset_collect_within_range_for_all_except(
    game: &mut Game,
    pos: Position,
    player: usize,
) {
    for p in 0..game.human_players_count() {
        if p == player {
            continue;
        }
        reset_collect_within_range(p, pos, game);
    }
}

pub(crate) fn reset_collection_stats(p: &mut Player) {
    for c in &mut p.cities {
        c.possible_collections.clear();
    }
}
