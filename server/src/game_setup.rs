use crate::action_card::gain_action_card_from_pile;
use crate::advance::Advance;
use crate::cache::Cache;
use crate::consts::{ACTIONS, JSON_SCHEMA_VERSION, NON_HUMAN_PLAYERS};
use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::content::{builtin, civilizations};
use crate::game::{Game, GameContext, GameOptions, GameState};
use crate::map::Map;
use crate::objective_card::gain_objective_card_from_pile;
use crate::player::{Player, add_unit};
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::utils::{Rng, Shuffle};
use itertools::Itertools;
use std::collections::HashMap;

#[must_use]
pub struct GameSetup {
    player_amount: usize,
    seed: String,
    random_map: bool,
    options: GameOptions,
    civilizations: Vec<String>,
}

#[must_use]
pub struct GameSetupBuilder {
    player_amount: usize,
    seed: String,
    random_map: bool,
    options: GameOptions,
    civilizations: Vec<String>,
}

impl GameSetupBuilder {
    pub fn new(player_amount: usize) -> Self {
        GameSetupBuilder {
            player_amount,
            seed: String::new(),
            random_map: true,
            options: GameOptions::default(),
            civilizations: Vec::new(),
        }
    }

    pub fn seed(mut self, seed: String) -> Self {
        self.seed = seed;
        self
    }

    pub fn skip_random_map(mut self) -> Self {
        self.random_map = false;
        self
    }

    pub fn options(mut self, options: GameOptions) -> Self {
        self.options = options;
        self
    }

    pub fn civilizations(mut self, civilizations: Vec<String>) -> Self {
        self.civilizations = civilizations;
        self
    }

    pub fn build(self) -> GameSetup {
        GameSetup {
            player_amount: self.player_amount,
            seed: self.seed,
            random_map: self.random_map,
            options: self.options,
            civilizations: self.civilizations,
        }
    }
}

/// Creates a new [`Game`].
///
/// # Panics
///
/// Panics only if there is an internal bug
#[must_use]
pub fn setup_game(setup: GameSetup) -> Game {
    setup_game_with_cache(setup, Cache::new())
}

/// Creates a new [`Game`].
///
/// # Panics
///
/// Panics only if there is an internal bug
#[must_use]
pub fn setup_game_with_cache(setup: GameSetup, cache: Cache) -> Game {
    let mut rng = init_rng(setup.seed.clone());

    let mut players = init_human_players(&setup, &mut rng);

    let starting_player = rng.range(0, players.len());

    players.push(Player::new(
        civilizations::get_civilization(BARBARIANS).expect("civ not found"),
        players.len(),
    ));
    players.push(Player::new(
        civilizations::get_civilization(PIRATES).expect("civ not found"),
        players.len(),
    ));

    let map = if setup.random_map {
        Map::random_map(&mut players, &mut rng)
    } else {
        Map::new(HashMap::new())
    };

    let wonders_left = cache
        .get_wonders()
        .iter()
        .map(|w| w.wonder)
        .collect_vec()
        .shuffled(&mut rng);
    let action_cards_left = cache
        .get_action_cards()
        .iter()
        .map(|a| a.id)
        .collect_vec()
        .shuffled(&mut rng);
    let objective_cards_left = cache
        .get_objective_cards()
        .iter()
        .map(|a| a.id)
        .collect_vec()
        .shuffled(&mut rng);
    let incidents_left = cache
        .get_incidents()
        .iter()
        .map(|i| i.id)
        .collect_vec()
        .shuffled(&mut rng);
    let all = &cache.get_builtins().clone();
    let mut game = Game {
        seed: setup.seed.clone(),
        context: GameContext::Play,
        version: JSON_SCHEMA_VERSION,
        options: setup.options,
        cache,
        state: GameState::Playing,
        events: Vec::new(),
        players,
        map,
        starting_player_index: starting_player,
        current_player_index: starting_player,
        action_log: Vec::new(),
        action_log_index: 0,
        log: [String::from("The game has started")]
            .iter()
            .map(|s| vec![s.clone()])
            .collect(),
        undo_limit: 0,
        actions_left: ACTIONS,
        successful_cultural_influence: false,
        round: 1,
        age: 0,
        messages: vec![],
        rng,
        dice_roll_outcomes: Vec::new(),
        dice_roll_log: Vec::new(),
        dropped_players: Vec::new(),
        wonders_left,
        action_cards_left,
        action_cards_discarded: Vec::new(),
        objective_cards_left,
        incidents_left,
        incidents_discarded: Vec::new(),
        permanent_effects: Vec::new(),
    };
    for i in 0..game.players.len() {
        builtin::init_player(&mut game, i, all);
    }

    for player_index in 0..setup.player_amount {
        let p = game.player(player_index);
        game.add_info_log_group(format!("{p} is playing as {}", p.civilization.name));
        gain_action_card_from_pile(&mut game, player_index);
        gain_objective_card_from_pile(&mut game, player_index);
        let p = game.player(player_index);
        if setup.random_map {
            add_unit(p.index, p.cities[0].position, UnitType::Settler, &mut game);
        }
    }

    game.next_age();
    game
}

fn init_rng(seed: String) -> Rng {
    let seed_length = seed.len();
    let seed = if seed_length < 32 {
        seed + &" ".repeat(32 - seed_length)
    } else {
        String::from(&seed[..32])
    };
    let seed: &[u8] = seed.as_bytes();
    let mut buffer = [0u8; 16];
    buffer[..].copy_from_slice(&seed[..16]);
    let seed1 = u128::from_be_bytes(buffer);
    let mut buffer = [0u8; 16];
    buffer[..].copy_from_slice(&seed[16..]);
    let seed2 = u128::from_be_bytes(buffer);
    let seed = seed1 ^ seed2;
    Rng::from_seed(seed)
}

fn init_human_players(setup: &GameSetup, rng: &mut Rng) -> Vec<Player> {
    let mut players = Vec::new();
    let mut civilizations = civilizations::get_all();
    for player_index in 0..setup.player_amount {
        let civilization = if setup.civilizations.is_empty() {
            civilizations.remove(rng.range(NON_HUMAN_PLAYERS, civilizations.len()))
        } else {
            rng.next_seed(); // need to call next_seed to have the same number of calls to rng
            civilizations::get_civilization(&setup.civilizations[player_index])
                .expect("civilization not found")
        };
        let mut player = Player::new(civilization, player_index);
        player.resource_limit = ResourcePile::new(2, 7, 7, 7, 7, 0, 0);
        player.gain_resources(ResourcePile::food(2));
        player.advances.insert(Advance::Farming);
        player.advances.insert(Advance::Mining);
        player.incident_tokens = 3;
        players.push(player);
    }
    players
}
