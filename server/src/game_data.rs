use crate::cache::Cache;
use crate::content::effects::PermanentEffect;
use crate::content::persistent_events::PersistentEventState;
use crate::game::{Game, GameContext, GameOptions, GameState};
use crate::log::ActionLogAge;
use crate::map::{Map, MapData};
use crate::player::{Player, PlayerData};
use crate::utils;
use crate::utils::Rng;
use crate::wonder::Wonder;
use num::Zero;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    #[serde(default)]
    options: GameOptions,
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
    action_log: Vec<ActionLogAge>,
    action_log_index: usize,
    log: Vec<Vec<String>>,
    undo_limit: usize,
    actions_left: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "utils::is_false")]
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
    };
    let all = game.cache.get_builtins().clone();
    for player in data.players {
        Player::initialize_player(player, &mut game, &all);
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
        players: game.players.into_iter().map(Player::data).collect(),
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
    }
}

#[must_use]
pub fn cloned_data(game: &Game) -> GameData {
    GameData {
        options: game.options.clone(),
        version: game.version,
        state: game.state.clone(),
        events: game.events.clone(),
        players: game.players.iter().map(Player::cloned_data).collect(),
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
    }
}

fn is_string_zero(s: &String) -> bool {
    s == "0"
}
