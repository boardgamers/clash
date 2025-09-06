use crate::advance::Advance;
use crate::city::activate_city;
use crate::content::custom_actions::custom_action_modifier_event_origin;
use crate::content::persistent_events::PersistentEventType;
use crate::events::EventOrigin;
use crate::game::Game;
use crate::map::Terrain;
use crate::map::Terrain::{Fertile, Forest, Mountain};
use crate::player::{CostTrigger, Player};
use crate::player_events::ActionInfo;
use crate::playing_actions::{PlayingActionType, base_or_modified_available};
use crate::position::Position;
use crate::resource::gain_resources_with_modifiers;
use crate::resource_pile::ResourcePile;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Debug)]
pub struct PositionCollection {
    pub position: Position,
    pub pile: ResourcePile,
    pub times: u8,
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
    pub fn times(&self, times: u8) -> PositionCollection {
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Collect {
    pub city_position: Position,
    pub collections: Vec<PositionCollection>,
    pub action_type: PlayingActionType,
}

impl Collect {
    #[must_use]
    pub fn new(
        city_position: Position,
        collections: Vec<PositionCollection>,
        action_type: PlayingActionType,
    ) -> Self {
        Self {
            city_position,
            collections,
            action_type,
        }
    }
}

///
/// # Errors
///
/// Errors if the action is illegal
pub fn get_total_collection(
    game: &Game,
    player_index: usize,
    origin: &EventOrigin,
    city_position: Position,
    collections: &[PositionCollection],
    trigger: CostTrigger,
) -> Result<CollectInfo, String> {
    let player = &game.players[player_index];
    let city = player.get_city(city_position);

    if city.player_index != player_index {
        return Err("Not your city".to_string());
    }

    let i = possible_resource_collections(game, city_position, player_index, origin, trigger);
    if i.max_selection < tiles_used(collections) {
        return Err(format!(
            "You can only collect {} resources at {city_position} - got {}",
            i.max_selection,
            tiles_used(collections)
        ));
    }
    let range2_tiles = collections
        .iter()
        .filter(|c| c.position.distance(i.city) > 1)
        .count();
    if range2_tiles > i.max_range2_tiles as usize {
        return Err(format!(
            "You can only collect {} resources from range 2 tiles - got {range2_tiles}",
            i.max_range2_tiles,
        ));
    }

    for (_, group) in &collections.iter().chunk_by(|c| c.position) {
        let used = group.map(|c| c.times).sum::<u8>();

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
                "You can only collect {:?} from {:?} - all options are {:?}",
                option, c.position, i.choices
            ));
        }
    }

    apply_total_collect(collections, player, i, game)
}

pub(crate) fn apply_total_collect(
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
    player.trigger_event(|e| &e.collect_total, &mut i, game, &collections.to_vec());
    Ok(i)
}

#[must_use]
pub fn tiles_used(collections: &[PositionCollection]) -> u8 {
    collections.iter().map(|c| c.times).sum()
}

pub(crate) fn execute_collect(
    game: &mut Game,
    player_index: usize,
    c: &Collect,
) -> Result<(), String> {
    let origin = collect_event_origin(&c.action_type, game.player(player_index));

    let mut i = get_total_collection(
        game,
        player_index,
        &origin,
        c.city_position,
        &c.collections,
        game.execute_cost_trigger(),
    )?;
    let city = game.players[player_index].get_city_mut(c.city_position);
    if !city.can_activate() {
        return Err("City can't be activated".to_string());
    }
    activate_city(city.position, game, &origin);
    gain_resources_with_modifiers(
        game,
        player_index,
        i.total.clone(),
        origin,
        i.modifiers.clone(),
    );

    let key = Advance::Husbandry.id();
    if i.info.info.contains_key(&key) {
        // check if husbandry is used
        let used = c
            .collections
            .iter()
            .any(|p| p.position.distance(c.city_position) > 1);
        if !used {
            i.info.info.remove(&key);
        }
    }

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
    pub terrain_options: HashMap<Terrain, HashSet<ResourcePile>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CollectInfo {
    pub choices: HashMap<Position, HashSet<ResourcePile>>,
    pub modifiers: Vec<EventOrigin>,
    pub city: Position,
    pub total: ResourcePile,
    pub max_per_tile: u8,
    pub max_selection: u8,
    pub max_range2_tiles: u8,
    pub(crate) info: ActionInfo,
}

impl CollectInfo {
    pub(crate) fn new(
        choices: HashMap<Position, HashSet<ResourcePile>>,
        player: &Player,
        origin: &EventOrigin,
        city: Position,
    ) -> CollectInfo {
        CollectInfo {
            choices,
            modifiers: Vec::new(),
            total: ResourcePile::empty(),
            info: ActionInfo::new(player, origin.clone()),
            city,
            max_per_tile: 1,
            max_selection: player.get_city(city).mood_modified_size(player) as u8,
            max_range2_tiles: 0,
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
    origin: &EventOrigin,
    trigger: CostTrigger,
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
    let modifiers =
        event
            .get()
            .trigger_with_modifiers(&mut terrain_options, &(), &(), &mut (), trigger);

    let collect_options = city_pos
        .neighbors()
        .into_iter()
        .chain(iter::once(city_pos))
        .filter_map(|pos| {
            if let Some(t) = game.map.get(pos)
                && let Some(option) = terrain_options.get(t)
            {
                return Some((pos, option.clone()));
            }
            None
        })
        .collect();

    let mut collect_info = CollectInfo::new(
        collect_options,
        &game.players[player_index],
        origin,
        city_pos,
    );
    let collect_context = CollectContext {
        player_index,
        city_position: city_pos,
        terrain_options,
    };

    game.player(player_index).trigger_event(
        |e| &e.collect_options,
        &mut collect_info,
        &collect_context,
        game,
    );

    collect_info.modifiers.extend(modifiers);
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
    base_or_modified_available(game, player, &PlayingActionType::Collect)
}

pub(crate) fn collect_event_origin(
    action_type: &PlayingActionType,
    player: &Player,
) -> EventOrigin {
    custom_action_modifier_event_origin(base_collect_event_origin(), action_type, player)
}

pub(crate) fn base_collect_event_origin() -> EventOrigin {
    EventOrigin::Ability("Collect".to_string())
}
