use crate::action_card::gain_action_card_from_pile;
use crate::advance::Advance;
use crate::cache::Cache;
use crate::consts::{ACTIONS, NON_HUMAN_PLAYERS};
use crate::content::civilizations::{BARBARIANS, PIRATES};
use crate::content::{builtin, civilizations};
use crate::game::{Game, GameState};
use crate::map::Map;
use crate::objective_card::gain_objective_card_from_pile;
use crate::player::{Player, add_unit};
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::utils::{Rng, Shuffle};
use itertools::Itertools;
use std::collections::HashMap;

/// Creates a new [`Game`].
///
/// # Panics
///
/// Panics only if there is an internal bug
#[must_use]
pub fn setup_game(player_amount: usize, seed: String, setup: bool) -> Game {
    setup_game_with_cache(player_amount, seed, setup, Cache::new())
}

/// Creates a new [`Game`].
///
/// # Panics
///
/// Panics only if there is an internal bug
#[must_use]
pub fn setup_game_with_cache(
    player_amount: usize,
    seed: String,
    setup: bool,
    cache: Cache,
) -> Game {
    let mut rng = init_rng(seed);
    let mut players = init_human_players(player_amount, &mut rng);

    let starting_player = rng.range(0, players.len());

    players.push(Player::new(
        civilizations::get_civilization(BARBARIANS).expect("civ not found"),
        players.len(),
    ));
    players.push(Player::new(
        civilizations::get_civilization(PIRATES).expect("civ not found"),
        players.len(),
    ));

    let map = if setup {
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
        ai_mode: false,
        actions_left: ACTIONS,
        successful_cultural_influence: false,
        round: 1,
        age: 0,
        messages: vec![String::from("The game has started")],
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

    for player_index in 0..player_amount {
        let p = game.player(player_index);
        game.add_info_log_group(format!(
            "{} is playing as {}",
            p.get_name(),
            p.civilization.name
        ));
        gain_action_card_from_pile(&mut game, player_index);
        gain_objective_card_from_pile(&mut game, player_index);
        let p = game.player(player_index);
        if setup {
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

fn init_human_players(player_amount: usize, rng: &mut Rng) -> Vec<Player> {
    let mut players = Vec::new();
    let mut civilizations = civilizations::get_all();
    for player_index in 0..player_amount {
        let civilization = rng.range(NON_HUMAN_PLAYERS, civilizations.len());
        let mut player = Player::new(civilizations.remove(civilization), player_index);
        player.resource_limit = ResourcePile::new(2, 7, 7, 7, 7, 0, 0);
        player.gain_resources(ResourcePile::food(2));
        player.advances.insert(Advance::Farming);
        player.advances.insert(Advance::Mining);
        player.incident_tokens = 3;
        players.push(player);
    }
    players
}
