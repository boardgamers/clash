use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::map::Terrain::{Fertile, Forest, Mountain};
use crate::player::Player;
use crate::player_events::ActionInfo;
use crate::playing_actions::Collect;
use crate::position::Position;
use crate::resource_pile::ResourcePile;
use std::collections::{HashMap, HashSet};
use std::iter;

///
/// # Panics
///
/// Panics if the action is illegal
#[must_use]
pub fn get_total_collection(
    game: &Game,
    player_index: usize,
    city_position: Position,
    collections: &[(Position, ResourcePile)],
) -> Option<(CollectOptionsInfo, ResourcePile)> {
    let player = &game.players[player_index];
    let city = player.get_city(city_position)?;
    if city.mood_modified_size() < collections.len() || city.player_index != player_index {
        return None;
    }
    let i = possible_resource_collections(
        game,
        city_position,
        player_index,
        &HashMap::new(),
        collections,
    );
    let possible = collections.iter().all(|(position, collect)| {
        i.choices
            .get(position)
            .is_some_and(|options| options.contains(collect))
    });

    if possible {
        collections
            .iter()
            .cloned()
            .map(|(_, collect)| collect)
            .reduce(std::ops::Add::add)
            .map(|option| (i, option))
    } else {
        None
    }
}

pub(crate) fn collect(game: &mut Game, player_index: usize, c: &Collect) {
    let (i, total_collect) =
        get_total_collection(game, player_index, c.city_position, &c.collections)
            .expect("Illegal action");
    let city = game.players[player_index]
        .get_city_mut(c.city_position)
        .expect("Illegal action");
    assert!(city.can_activate(), "Illegal action");
    city.activate();
    game.players[player_index].gain_resources(total_collect);
    i.execute(game);
}

pub(crate) fn undo_collect(game: &mut Game, player_index: usize, c: Collect) {
    game.players[player_index]
        .get_city_mut(c.city_position)
        .expect("city should be owned by the player")
        .undo_activate();
    let total_collect = c.collections.into_iter().map(|(_, collect)| collect).sum();
    game.players[player_index].lose_resources(total_collect);
}

pub(crate) struct CollectContext {
    pub player_index: usize,
    pub city_position: Position,
    pub used: HashMap<Position, ResourcePile>,
    pub terrain_options: HashMap<Terrain, HashSet<ResourcePile>>,
}

#[derive(Clone, PartialEq)]
pub struct CollectOptionsInfo {
    pub choices: HashMap<Position, HashSet<ResourcePile>>,
    pub modifiers: Vec<EventOrigin>,
    pub(crate) info: ActionInfo,
}

impl CollectOptionsInfo {
    pub(crate) fn new(
        choices: HashMap<Position, HashSet<ResourcePile>>,
        player: &Player,
    ) -> CollectOptionsInfo {
        CollectOptionsInfo {
            choices,
            modifiers: Vec::new(),
            info: ActionInfo::new(player),
        }
    }

    pub(crate) fn execute(&self, game: &mut Game) {
        self.info.execute(game);
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
    used: &HashMap<Position, ResourcePile>,
    expect: &[(Position, ResourcePile)],
) -> CollectOptionsInfo {
    let set = [
        (Mountain, HashSet::from([ResourcePile::ore(1)])),
        (Fertile, HashSet::from([ResourcePile::food(1)])),
        (Forest, HashSet::from([ResourcePile::wood(1)])),
    ];
    let mut terrain_options = HashMap::from(set);
    let modifiers = game.players[player_index]
        .events
        .terrain_collect_options
        .get()
        .trigger(&mut terrain_options, &(), &());

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
        .collect_options
        .get()
        .trigger_with_minimal_modifiers(
            &CollectOptionsInfo::new(collect_options, &game.players[player_index]),
            &CollectContext {
                player_index,
                city_position: city_pos,
                used: used.clone(),
                terrain_options,
            },
            game,
            |i| {
                expect.is_empty()
                    || i.choices.iter().all(|(pos, options)| {
                        expect
                            .iter()
                            .any(|(e_pos, e_option)| pos == e_pos && options.contains(e_option))
                    })
            },
            |i, m| i.modifiers = m,
        );

    i.modifiers.extend(modifiers);

    for (pos, pile) in used {
        i.choices.entry(*pos).or_default().insert(pile.clone());
    }

    i.choices.retain(|p, _| {
        game.get_any_city(*p).is_none_or(|c| c.position == city_pos)
            && game.enemy_player(player_index, *p).is_none()
    });
    i
}
