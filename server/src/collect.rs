use crate::content::custom_phase_actions::CurrentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::map::Terrain::{Fertile, Forest, Mountain};
use crate::player::Player;
use crate::player_events::ActionInfo;
use crate::playing_actions::Collect;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
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

    if city.mood_modified_size(player) < tiles_used(collections) as usize {
        return Err(format!(
            "You can only collect {} resources - got {}",
            city.mood_modified_size(player),
            tiles_used(collections)
        ));
    }
    let mut i =
        possible_resource_collections(game, city_position, player_index, &Vec::new(), collections);

    if collections.iter().any(|c| c.times > i.max_per_tile) {
        return Err(format!(
            "You can only collect {} resources from each tile",
            i.max_per_tile,
        ));
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

    collections
        .iter()
        .cloned()
        .map(|c| c.total())
        .reduce(std::ops::Add::add)
        .map(|pile| {
            i.total = pile;
            let _ = player.trigger_event(|e| &e.collect_total, &mut i, &(), &());
            i
        })
        .ok_or("Nothing collected".to_string())
}

#[must_use]
pub fn tiles_used(collections: &[PositionCollection]) -> u32 {
    collections.iter().map(|c| c.times).sum()
}

pub(crate) fn collect(game: &mut Game, player_index: usize, c: &Collect) {
    let i = get_total_collection(game, player_index, c.city_position, &c.collections)
        .unwrap_or_else(|e| panic!("{e}"));
    let city = game.players[player_index].get_city_mut(c.city_position);
    assert!(city.can_activate(), "Illegal action");
    city.activate();
    game.players[player_index].gain_resources(i.total.clone());

    on_collect(game, player_index, i);
}

pub(crate) fn on_collect(game: &mut Game, player_index: usize, i: CollectInfo) {
    let Some(info) = game.trigger_current_event(
        &[player_index],
        |e| &mut e.on_collect,
        i,
        CurrentEventType::Collect,
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
    min: &[PositionCollection],
) -> CollectInfo {
    let set = [
        (Mountain, HashSet::from([ResourcePile::ore(1)])),
        (Fertile, HashSet::from([ResourcePile::food(1)])),
        (Forest, HashSet::from([ResourcePile::wood(1)])),
    ];
    let mut terrain_options = HashMap::from(set);
    let modifiers = game.get_player(player_index).trigger_event(
        |e| &e.terrain_collect_options,
        &mut terrain_options,
        &(),
        &(),
    );

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

    let mut i = game.players[player_index]
        .events
        .transient
        .collect_options
        .get()
        .trigger_with_minimal_modifiers(
            &CollectInfo::new(collect_options, &game.players[player_index], city_pos),
            &CollectContext {
                player_index,
                city_position: city_pos,
                used: used.to_vec(),
                terrain_options,
            },
            game,
            &mut (),
            |i| {
                if min.is_empty() {
                    // always get as many as possible if no expectations
                    return false;
                }
                i.choices.iter().all(|(pos, options)| {
                    min.iter()
                        .any(|e| pos == &e.position && options.contains(&e.pile))
                })
            },
            |i, m| i.modifiers = m,
        );

    i.modifiers.extend(modifiers);

    for u in used {
        i.choices
            .entry(u.position)
            .or_default()
            .insert(u.pile.clone());
    }

    i.choices.retain(|p, _| {
        game.try_get_any_city(*p)
            .is_none_or(|c| c.position == city_pos)
            && game.enemy_player(player_index, *p).is_none()
    });
    i
}
