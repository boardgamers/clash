use crate::advance;
use crate::advance::{Advance, is_special_advance_active};
use crate::cache::Cache;
use crate::city::{City, CityData};
use crate::city_pieces::{DestroyedStructures, DestroyedStructuresData};
use crate::content::ability;
use crate::content::ability::Ability;
use crate::content::custom_actions::CustomActionType;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::PersistentEventState;
use crate::game::{Game, GameContext, GameOptions, GameState, UIElement};
use crate::leader::Leader;
use crate::log::ActionLogAge;
use crate::map::{Map, MapData};
use crate::objective_card::{CompletedObjective, init_objective_card};
use crate::player::{Data, Player};
use crate::player_events::PlayerEvents;
use crate::resource_pile::ResourcePile;
use crate::unit::{Unit, UnitData};
use crate::utils::Rng;
use crate::victory_points::SpecialVictoryPoints;
use crate::wonder::{Wonder, init_wonder};
use enumset::EnumSet;
use itertools::Itertools;
use num::Zero;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem;

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    #[serde(default)]
    #[serde(skip_serializing_if = "GameOptions::is_default")]
    pub options: GameOptions,
    #[serde(default)]
    #[serde(skip_serializing_if = "u16::is_zero")]
    version: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    seed: String,
    state: GameState,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<PersistentEventState>,
    players: Vec<PlayerData>,
    map: MapData,
    starting_player_index: usize,
    current_player_index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub action_log: Vec<ActionLogAge>,
    action_log_index: usize,
    log: Vec<Vec<String>>,
    undo_limit: usize,
    actions_left: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    successful_cultural_influence: bool,
    round: u32,
    age: u32,
    messages: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dice_roll_outcomes: Vec<u8>, // for testing purposes
    #[serde(default)]
    #[serde(skip_serializing_if = "is_string_zero")]
    rng: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dice_roll_log: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dropped_players: Vec<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonders_left: Vec<Wonder>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_cards_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_cards_discarded: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    objective_cards_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    incidents_left: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    incidents_discarded: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    permanent_effects: Vec<PermanentEffect>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    custom_ui_elements: HashMap<String, UIElement>,
}

///
///
/// # Panics
///
/// Panics if any wonder does not exist
#[must_use]
pub fn from_data(data: GameData, cache: Cache, context: GameContext) -> Game {
    let mut game = Game {
        context,
        cache,
        options: data.options,
        version: data.version,
        state: data.state,
        players: Vec::new(),
        map: Map::from_data(data.map),
        starting_player_index: data.starting_player_index,
        current_player_index: data.current_player_index,
        actions_left: data.actions_left,
        successful_cultural_influence: data.successful_cultural_influence,
        action_log: data.action_log,
        action_log_index: data.action_log_index,
        log: data.log,
        undo_limit: data.undo_limit,
        round: data.round,
        age: data.age,
        messages: data.messages,
        seed: data.seed,
        rng: Rng::from_seed_string(&data.rng),
        dice_roll_outcomes: data.dice_roll_outcomes,
        dice_roll_log: data.dice_roll_log,
        dropped_players: data.dropped_players,
        wonders_left: data.wonders_left,
        action_cards_left: data.action_cards_left,
        action_cards_discarded: data.action_cards_discarded,
        objective_cards_left: data.objective_cards_left,
        incidents_left: data.incidents_left,
        incidents_discarded: data.incidents_discarded,
        permanent_effects: data.permanent_effects,
        events: data.events,
        custom_ui_elements: data.custom_ui_elements,
    };
    for player_data in data.players {
        game.players.push(player_from_data(player_data, &game));
    }
    let all = game.cache.get_abilities().clone();
    for player_index in 0..game.players.len() {
        initialize_player(player_index, &mut game, &all);
    }
    game
}

#[must_use]
pub fn data(game: Game) -> GameData {
    GameData {
        options: game.options,
        version: game.version,
        state: game.state,
        events: game.events,
        players: game.players.into_iter().map(player_data).collect(),
        map: game.map.data(),
        starting_player_index: game.starting_player_index,
        current_player_index: game.current_player_index,
        action_log: game.action_log,
        action_log_index: game.action_log_index,
        log: game.log,
        undo_limit: game.undo_limit,
        actions_left: game.actions_left,
        successful_cultural_influence: game.successful_cultural_influence,
        round: game.round,
        age: game.age,
        messages: game.messages,
        seed: game.seed,
        rng: game.rng.seed.to_string(),
        dice_roll_outcomes: game.dice_roll_outcomes,
        dice_roll_log: game.dice_roll_log,
        dropped_players: game.dropped_players,
        wonders_left: game.wonders_left,
        action_cards_left: game.action_cards_left,
        action_cards_discarded: game.action_cards_discarded,
        objective_cards_left: game.objective_cards_left,
        incidents_left: game.incidents_left,
        incidents_discarded: game.incidents_discarded,
        permanent_effects: game.permanent_effects,
        custom_ui_elements: game.custom_ui_elements,
    }
}

#[must_use]
pub fn cloned_data(game: &Game) -> GameData {
    GameData {
        options: game.options.clone(),
        version: game.version,
        state: game.state.clone(),
        events: game.events.clone(),
        players: game.players.iter().map(cloned_player_data).collect(),
        map: game.map.cloned_data(),
        starting_player_index: game.starting_player_index,
        current_player_index: game.current_player_index,
        action_log: game.action_log.clone(),
        action_log_index: game.action_log_index,
        log: game.log.clone(),
        undo_limit: game.undo_limit,
        actions_left: game.actions_left,
        successful_cultural_influence: game.successful_cultural_influence,
        round: game.round,
        age: game.age,
        messages: game.messages.clone(),
        seed: game.seed.clone(),
        rng: game.rng.seed.to_string(),
        dice_roll_outcomes: game.dice_roll_outcomes.clone(),
        dice_roll_log: game.dice_roll_log.clone(),
        dropped_players: game.dropped_players.clone(),
        wonders_left: game.wonders_left.clone(),
        action_cards_left: game.action_cards_left.clone(),
        action_cards_discarded: game.action_cards_discarded.clone(),
        objective_cards_left: game.objective_cards_left.clone(),
        incidents_left: game.incidents_left.clone(),
        incidents_discarded: game.incidents_discarded.clone(),
        permanent_effects: game.permanent_effects.clone(),
        custom_ui_elements: game.custom_ui_elements.clone(),
    }
}

fn is_string_zero(s: &String) -> bool {
    s == "0"
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct PlayerData {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    id: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    resources: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    resource_limit: ResourcePile,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cities: Vec<CityData>,
    #[serde(default)]
    #[serde(skip_serializing_if = "DestroyedStructuresData::is_empty")]
    destroyed_structures: DestroyedStructuresData,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    units: Vec<UnitData>,
    civilization: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    available_leaders: Vec<Leader>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    recruited_leaders: Vec<Leader>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    advances: Vec<Advance>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    great_library_advance: Option<Advance>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonders_built: Vec<Wonder>,
    #[serde(default)]
    #[serde(skip_serializing_if = "u8::is_zero")]
    incident_tokens: u8,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    completed_objectives: Vec<CompletedObjective>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    captured_leaders: Vec<Leader>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    special_victory_points: Vec<SpecialVictoryPoints>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wonder_cards: Vec<Wonder>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    action_cards: Vec<u8>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    objective_cards: Vec<u8>,
    next_unit_id: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    played_once_per_turn_actions: Vec<CustomActionType>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    event_info: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    secrets: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    custom_data: HashMap<String, Data>,
}

///
///
/// # Panics
///
/// Panics if elements like wonders or advances don't exist
fn initialize_player(player_index: usize, game: &mut Game, builtin_abilities: &[Ability]) {
    ability::init_player(game, player_index, builtin_abilities);
    advance::init_player(game, player_index);

    if let Some(leader) = game.player(player_index).active_leader() {
        Player::with_leader(leader, game, player_index, |game, leader| {
            leader.listeners.init(game, player_index);
        });
    }

    for id in game.player(player_index).objective_cards.clone() {
        if id == 0 {
            // hidden
            continue;
        }
        init_objective_card(game, player_index, id);
    }

    let mut cities = mem::take(&mut game.players[player_index].cities);
    for city in &mut cities {
        for wonder in &city.pieces.wonders {
            init_wonder(game, player_index, *wonder);
        }
    }
    game.players[player_index].cities = cities;
}

fn player_from_data(data: PlayerData, game: &Game) -> Player {
    let units = data
        .units
        .into_iter()
        .flat_map(|u| Unit::from_data(data.id, u))
        .collect_vec();
    units
        .iter()
        .into_group_map_by(|unit| unit.id)
        .iter()
        .for_each(|(id, units)| {
            assert_eq!(
                units.len(),
                1,
                "player data {} should not contain duplicate units {id}",
                data.id
            );
        });
    let cities = data
        .cities
        .into_iter()
        .map(|d| City::from_data(d, data.id))
        .collect_vec();
    let advances = EnumSet::from_iter(data.advances);
    let civilization = game.cache.get_civilization(&data.civilization);
    Player {
        name: data.name,
        index: data.id,
        resources: data.resources,
        resource_limit: data.resource_limit,
        wasted_resources: ResourcePile::empty(),
        events: PlayerEvents::new(),
        destroyed_structures: DestroyedStructures::from_data(data.destroyed_structures),
        units,
        available_leaders: data.available_leaders,
        recruited_leaders: data.recruited_leaders,
        great_library_advance: data.great_library_advance,
        special_advances: civilization
            .special_advances
            .iter()
            .filter(|s| is_special_advance_active(s.advance, advances, game))
            .map(|s| s.advance)
            .collect(),
        civilization,
        advances,
        wonders_built: data.wonders_built,
        wonders_owned: cities
            .iter()
            .flat_map(|city| city.pieces.wonders.iter().copied())
            .collect(),
        cities,
        incident_tokens: data.incident_tokens,
        completed_objectives: data.completed_objectives,
        captured_leaders: data.captured_leaders,
        special_victory_points: data.special_victory_points,
        custom_actions: HashMap::new(),
        wonder_cards: data.wonder_cards,
        action_cards: data.action_cards,
        objective_cards: data.objective_cards,
        next_unit_id: data.next_unit_id,
        played_once_per_turn_actions: data.played_once_per_turn_actions,
        event_info: data.event_info,
        secrets: data.secrets,
        custom_data: data.custom_data,
        objective_opportunities: Vec::new(),
        gained_objective: None,
        great_mausoleum_action_cards: 0,
    }
}

#[must_use]
pub fn player_data(player: Player) -> PlayerData {
    let units = player
        .units
        .iter()
        // carried units are added to carriers
        .filter(|unit| {
            if let Some(carrier_id) = unit.carrier_id {
                // safety check
                let _ = player.get_unit(carrier_id);
            }
            unit.carrier_id.is_none()
        })
        .sorted_by_key(|unit| unit.id)
        .map(|u| u.data(&player))
        .collect();
    PlayerData {
        name: player.name,
        id: player.index,
        resources: player.resources,
        resource_limit: player.resource_limit,
        cities: player.cities.into_iter().map(City::data).collect(),
        destroyed_structures: player.destroyed_structures.data(),
        units,
        civilization: player.civilization.name,
        available_leaders: player.available_leaders,
        recruited_leaders: player.recruited_leaders,
        advances: player.advances.iter().sorted_by_key(Advance::id).collect(),
        great_library_advance: player.great_library_advance,
        wonders_built: player.wonders_built,
        incident_tokens: player.incident_tokens,
        completed_objectives: player.completed_objectives,
        captured_leaders: player.captured_leaders,
        special_victory_points: player.special_victory_points,
        wonder_cards: player.wonder_cards,
        action_cards: player.action_cards,
        objective_cards: player.objective_cards,
        next_unit_id: player.next_unit_id,
        played_once_per_turn_actions: player.played_once_per_turn_actions,
        event_info: player.event_info,
        secrets: player.secrets,
        custom_data: player.custom_data,
    }
}

pub fn cloned_player_data(player: &Player) -> PlayerData {
    let units = player
        .units
        .iter()
        // carried units are added to carriers
        .filter(|unit| unit.carrier_id.is_none())
        .sorted_by_key(|unit| unit.id)
        .map(|u| u.data(player))
        .collect();
    PlayerData {
        name: player.name.clone(),
        id: player.index,
        resources: player.resources.clone(),
        resource_limit: player.resource_limit.clone(),
        cities: player.cities.iter().map(City::cloned_data).collect(),
        destroyed_structures: player.destroyed_structures.cloned_data(),
        units,
        civilization: player.civilization.name.clone(),
        available_leaders: player.available_leaders.clone(),
        recruited_leaders: player.recruited_leaders.clone(),
        advances: player.advances.iter().sorted_by_key(Advance::id).collect(),
        great_library_advance: player.great_library_advance,
        wonders_built: player.wonders_built.clone(),
        incident_tokens: player.incident_tokens,
        completed_objectives: player.completed_objectives.clone(),
        captured_leaders: player.captured_leaders.clone(),
        special_victory_points: player.special_victory_points.clone(),
        wonder_cards: player.wonder_cards.clone(),
        action_cards: player.action_cards.clone(),
        objective_cards: player.objective_cards.clone(),
        next_unit_id: player.next_unit_id,
        played_once_per_turn_actions: player.played_once_per_turn_actions.clone(),
        event_info: player.event_info.clone(),
        secrets: player.secrets.clone(),
        custom_data: player.custom_data.clone(),
    }
}
