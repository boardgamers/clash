use crate::ability_initializer::AbilityInitializerSetup;
use crate::advance::Advance;
use crate::city::City;
use crate::content::builtin::Builtin;
use crate::content::custom_actions::CustomActionType;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::map::Terrain::{Fertile, Forest, Mountain};
use crate::player::Player;
use crate::player_events::ActionInfo;
use crate::playing_actions::{Collect, PlayingActionType, base_or_custom_available};
use crate::position::Position;
use crate::resource::ResourceType;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Debug)]
pub struct PositionCollection {
    pub position: Position,
    pub pile: ResourcePile,
    pub times: u32,
}

impl PositionCollection {
    #[must_use]
    pub fn new(position: Position, pile: ResourcePile) -> PositionCollection {
        PositionCollection {
            position,
            pile,
            times: 1,
        }
    }

    #[must_use]
    pub fn times(&self, times: u32) -> PositionCollection {
        PositionCollection {
            position: self.position,
            pile: self.pile.clone(),
            times,
        }
    }

    #[must_use]
    pub fn total(&self) -> ResourcePile {
        self.pile.times(self.times)
    }
}

///
/// # Errors
///
/// Errors if the action is illegal
pub fn get_total_collection(
    game: &Game,
    player_index: usize,
    city_position: Position,
    collections: &[PositionCollection],
) -> Result<CollectInfo, String> {
    let player = &game.players[player_index];
    let city = player.get_city(city_position);

    if city.player_index != player_index {
        return Err("Not your city".to_string());
    }

    let i = possible_resource_collections(game, city_position, player_index, &Vec::new());
    if i.max_selection < tiles_used(collections) {
        return Err(format!(
            "You can only collect {} resources at {city_position} - got {}",
            i.max_selection,
            tiles_used(collections)
        ));
    }

    for (_, group) in &collections.iter().chunk_by(|c| c.position) {
        let used = group.map(|c| c.times).sum::<u32>();

        if used > i.max_per_tile {
            return Err(format!(
                "You can only collect {} resources from each tile",
                i.max_per_tile,
            ));
        }
    }

    for c in collections {
        let option = i.choices.get(&c.position);
        if option.is_none_or(|options| !options.contains(&c.pile)) {
            return Err(format!(
                "You can only collect {:?} from {:?}",
                option, c.position
            ));
        }
    }

    apply_total_collect(collections, player, i, game)
}

fn apply_total_collect(
    collections: &[PositionCollection],
    player: &Player,
    mut i: CollectInfo,
    game: &Game,
) -> Result<CollectInfo, String> {
    let Some(total) = collections
        .iter()
        .cloned()
        .map(|c| c.total())
        .reduce(std::ops::Add::add)
    else {
        return Err("Nothing collected".to_string());
    };

    i.total = total;
    player.trigger_event(|e| &e.collect_total, &mut i, game, &());
    Ok(i)
}

#[must_use]
pub fn tiles_used(collections: &[PositionCollection]) -> u32 {
    collections.iter().map(|c| c.times).sum()
}

pub(crate) fn collect(game: &mut Game, player_index: usize, c: &Collect) -> Result<(), String> {
    let i = get_total_collection(game, player_index, c.city_position, &c.collections)?;
    let city = game.players[player_index].get_city_mut(c.city_position);
    if !city.can_activate() {
        return Err("City can't be activated".to_string());
    }
    city.activate();
    game.players[player_index].gain_resources(i.total.clone());

    on_collect(game, player_index, i);
    Ok(())
}

pub(crate) fn on_collect(game: &mut Game, player_index: usize, i: CollectInfo) {
    let Some(info) = game.trigger_persistent_event(
        &[player_index],
        |e| &mut e.collect,
        i,
        PersistentEventType::Collect,
    ) else {
        return;
    };

    info.info.execute(game);
}

pub(crate) struct CollectContext {
    pub player_index: usize,
    pub city_position: Position,
    pub used: Vec<PositionCollection>,
    pub terrain_options: HashMap<Terrain, HashSet<ResourcePile>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CollectInfo {
    pub choices: HashMap<Position, HashSet<ResourcePile>>,
    pub modifiers: Vec<EventOrigin>,
    pub city: Position,
    pub total: ResourcePile,
    pub max_per_tile: u32,
    pub max_selection: u32,
    pub(crate) info: ActionInfo,
}

impl CollectInfo {
    pub(crate) fn new(
        choices: HashMap<Position, HashSet<ResourcePile>>,
        player: &Player,
        city: Position,
    ) -> CollectInfo {
        CollectInfo {
            choices,
            modifiers: Vec::new(),
            total: ResourcePile::empty(),
            info: ActionInfo::new(player),
            city,
            max_per_tile: 1,
            max_selection: player.get_city(city).mood_modified_size(player) as u32,
        }
    }
}

///
/// # Panics
/// Panics if the action is illegal
#[must_use]
pub fn possible_resource_collections(
    game: &Game,
    city_pos: Position,
    player_index: usize,
    used: &[PositionCollection],
) -> CollectInfo {
    let set = [
        (Mountain, HashSet::from([ResourcePile::ore(1)])),
        (Fertile, HashSet::from([ResourcePile::food(1)])),
        (Forest, HashSet::from([ResourcePile::wood(1)])),
    ];
    let mut terrain_options = HashMap::from(set);
    let event = &game
        .player(player_index)
        .events
        .transient
        .terrain_collect_options;
    let modifiers = event
        .get()
        .trigger_with_modifiers(&mut terrain_options, &(), &(), &mut ());

    let collect_options = city_pos
        .neighbors()
        .into_iter()
        .chain(iter::once(city_pos))
        .filter_map(|pos| {
            if let Some(t) = game.map.get(pos) {
                if let Some(option) = terrain_options.get(t) {
                    return Some((pos, option.clone()));
                }
            }
            None
        })
        .collect();

    let mut collect_info = CollectInfo::new(collect_options, &game.players[player_index], city_pos);
    let collect_context = CollectContext {
        player_index,
        city_position: city_pos,
        used: used.to_vec(),
        terrain_options,
    };

    game.player(player_index).trigger_event(
        |e| &e.collect_options,
        &mut collect_info,
        &collect_context,
        game,
    );

    collect_info.modifiers.extend(modifiers);

    for u in used {
        collect_info
            .choices
            .entry(u.position)
            .or_default()
            .insert(u.pile.clone());
    }

    collect_info.choices.retain(|p, _| {
        game.try_get_any_city(*p)
            .is_none_or(|c| c.position == city_pos)
            && game.enemy_player(player_index, *p).is_none()
    });
    collect_info
}

#[must_use]
pub fn add_collect(
    info: &CollectInfo,
    collect_position: Position,
    pile: &ResourcePile,
    current: &[PositionCollection],
) -> Vec<PositionCollection> {
    let old = current
        .iter()
        .position(|old| old.position == collect_position && &old.pile == pile);

    let mut new: Vec<PositionCollection> = current.to_vec();

    if let Some(i) = old {
        if new[i].times < info.max_per_tile {
            new[i].times += 1;
        } else {
            new.remove(i);
        }
    } else {
        new.push(PositionCollection::new(collect_position, pile.clone()));
    }

    new
}

#[must_use]
pub fn available_collect_actions_for_city(
    game: &Game,
    player: usize,
    position: Position,
) -> Vec<PlayingActionType> {
    if game.player(player).get_city(position).can_activate() {
        available_collect_actions(game, player)
    } else {
        vec![]
    }
}

#[must_use]
pub fn available_collect_actions(game: &Game, player: usize) -> Vec<PlayingActionType> {
    base_or_custom_available(
        game,
        player,
        PlayingActionType::Collect,
        &CustomActionType::FreeEconomyCollect,
    )
}

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
    let info = possible_resource_collections(game, city.position, player.index, &[]);

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
        let info = possible_resource_collections(game, city.position, player.index, &c);

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
        .map(|(p, group)| (p, group.map(|c| c.times).sum::<u32>()))
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
                reset_collect_within_range(game.player_mut(player), *p);
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

pub(crate) fn reset_collect_within_range(p: &mut Player, position: Position) {
    let has_husbandry = p.has_advance(Advance::Husbandry);
    let range = if has_husbandry { 2 } else { 1 };
    for c in &mut p.cities {
        if c.position.distance(position) <= range {
            c.possible_collections.clear();
        }
    }
}

pub(crate) fn reset_collect_within_range_for_all(game: &mut Game, pos: Position) {
    for p in &mut game.players {
        reset_collect_within_range(p, pos);
    }
}

pub(crate) fn reset_collection_stats(p: &mut Player) {
    for c in &mut p.cities {
        c.possible_collections.clear();
    }
}
