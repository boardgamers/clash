use crate::action_card::gain_action_card_from_pile;
use crate::advance::{Advance, do_advance};
use crate::cache::Cache;
use crate::consts::{ACTIONS, JSON_SCHEMA_VERSION, NON_HUMAN_PLAYERS};
use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::content::{ability, civilizations};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::{Game, GameContext, GameOptions, GameState};
use crate::log::{add_player_log, add_round_log};
use crate::map::Map;
use crate::objective_card::gain_objective_card_from_pile;
use crate::player::{Player, gain_unit};
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
pub fn setup_game(setup: &GameSetup) -> Game {
    setup_game_with_cache(setup, Cache::new())
}

/// Creates a new [`Game`].
///
/// # Panics
///
/// Panics only if there is an internal bug
#[must_use]
pub fn setup_game_with_cache(setup: &GameSetup, cache: Cache) -> Game {
    let mut rng = init_rng(setup.seed.clone());

    let mut players = create_human_players(setup, &mut rng, &cache);

    let starting_player = rng.range(0, players.len());

    players.push(Player::new(
        cache.get_civilization(BARBARIANS),
        players.len(),
    ));
    players.push(Player::new(cache.get_civilization(PIRATES), players.len()));

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
    let all = &cache.get_abilities().clone();
    let mut game = Game {
        seed: setup.seed.clone(),
        context: GameContext::Play,
        version: JSON_SCHEMA_VERSION,
        options: setup.options.clone(),
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
        ability::init_player(&mut game, i, all);
    }

    execute_setup_round(setup, &mut game);
    game
}

fn execute_setup_round(setup: &GameSetup, game: &mut Game) {
    game.next_age();
    add_round_log(game, 0);

    for player_index in 0..setup.player_amount {
        add_player_log(game, player_index);

        let origin = setup_event_origin();
        let player = EventPlayer::from_player(player_index, game, origin.clone());
        player.log(
            game,
            &format!("Play as {}", game.player(player_index).civilization.name),
        );

        player.gain_resources(game, ResourcePile::food(2));
        do_advance(game, Advance::Farming, &player, false);
        do_advance(game, Advance::Mining, &player, false);

        gain_action_card_from_pile(game, &player);
        gain_objective_card_from_pile(game, &player);
        if setup.random_map {
            let home = game.player(player_index).cities[0].position;
            gain_unit(game, player_index, home, UnitType::Settler, &origin);
        }
    }
}

fn setup_event_origin() -> EventOrigin {
    EventOrigin::Ability("Setup".to_string())
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

fn create_human_players(setup: &GameSetup, rng: &mut Rng, cache: &Cache) -> Vec<Player> {
    let mut players = Vec::new();
    let mut civilizations = civilizations::get_all_uncached();
    for player_index in 0..setup.player_amount {
        let civilization = if setup.civilizations.is_empty() {
            civilizations.remove(rng.range(NON_HUMAN_PLAYERS, civilizations.len()))
        } else {
            rng.next_seed(); // need to call next_seed to have the same number of calls to rng
            cache.get_civilization(&setup.civilizations[player_index])
        };
        let mut player = Player::new(civilization, player_index);
        player.resource_limit = ResourcePile::new(2, 7, 7, 7, 7, 0, 0);
        player.incident_tokens = 3;
        players.push(player);
    }
    players
}
