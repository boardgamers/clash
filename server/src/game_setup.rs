use crate::action::Action;
use crate::action_card::gain_action_card_from_pile;
use crate::advance::{Advance, do_advance};
use crate::cache::Cache;
use crate::city;
use crate::city::{City, MoodState, set_city_mood};
use crate::civilization::Civilization;
use crate::consts::{ACTIONS, JSON_SCHEMA_VERSION};
use crate::content::ability;
use crate::content::civilizations::{BARBARIANS, CHOOSE_CIV, PIRATES};
use crate::events::{EventOrigin, EventPlayer};
use crate::game::{CivSetupOption, Game, GameContext, GameOptions, GameState};
use crate::leader::Leader;
use crate::log::{
    ActionLogAge, ActionLogRound, SetupTurnType, TurnType, add_start_turn_action_if_needed,
    add_turn_log,
};
use crate::map::{Map, MapSetup, get_map_setup};
use crate::objective_card::gain_objective_card_from_pile;
use crate::player::{Player, gain_unit};
use crate::resource_pile::ResourcePile;
use crate::unit::UnitType;
use crate::utils::{Rng, Shuffle};
use city::gain_city;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Persist the deal and locks, rather than re-dealing from a seed on reconnect.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct CivilizationDraft {
    pub offers: Vec<Vec<String>>,
    pub choices: Vec<Option<String>>,
    pub ready: Vec<bool>,
    #[serde(default)]
    pub live_update: bool,
    random_map: bool,
}

#[must_use]
pub struct GameSetup {
    player_amount: usize,
    seed: String,
    random_map: bool,
    options: GameOptions,
    assigned_civilizations: Vec<String>,
}

#[must_use]
pub struct GameSetupBuilder {
    player_amount: usize,
    seed: String,
    random_map: bool,
    options: GameOptions,
    assigned_civilizations: Vec<String>,
}

impl GameSetupBuilder {
    pub fn new(player_amount: usize) -> Self {
        GameSetupBuilder {
            player_amount,
            seed: String::new(),
            random_map: true,
            options: GameOptions::default(),
            assigned_civilizations: Vec::new(),
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

    pub fn assigned_civilizations(mut self, civilizations: Vec<String>) -> Self {
        self.assigned_civilizations = civilizations;
        self
    }

    pub fn build(self) -> GameSetup {
        GameSetup {
            player_amount: self.player_amount,
            seed: self.seed,
            random_map: self.random_map,
            options: self.options,
            assigned_civilizations: self.assigned_civilizations,
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
    setup_game_with_cache(setup, Cache::new(&setup.options))
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
    let civilization_draft = if setup.options.civilization == CivSetupOption::DraftThree {
        let mut pool = cache
            .get_civilizations()
            .iter()
            .filter(|c| c.can_choose())
            .map(|c| c.name.clone())
            .collect_vec();
        assert!(
            pool.len() >= setup.player_amount * 3,
            "Not enough civilizations for three per player"
        );
        pool.shuffle(&mut rng);
        Some(CivilizationDraft {
            offers: pool
                .chunks_exact(3)
                .take(setup.player_amount)
                .map(<[String]>::to_vec)
                .collect(),
            choices: vec![None; setup.player_amount],
            ready: vec![false; setup.player_amount],
            live_update: false,
            random_map: setup.random_map,
        })
    } else {
        None
    };

    players.push(Player::new(
        cache.get_civilization(BARBARIANS),
        players.len(),
    ));
    players.push(Player::new(cache.get_civilization(PIRATES), players.len()));

    let (map_setup, map) = if setup.random_map && civilization_draft.is_none() {
        let setup = get_map_setup(setup.player_amount);
        let map = Map::random_map(&mut rng, &setup);
        (Some(setup), map)
    } else {
        (None, Map::new(HashMap::new()))
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
        options: GameOptions {
            legacy_undo: None,
            ..setup.options.clone()
        },
        cache,
        state: if setup.options.civilization != CivSetupOption::Random {
            GameState::ChooseCivilization
        } else {
            GameState::Playing
        },
        events: Vec::new(),
        players,
        map,
        civilization_draft,
        starting_player_index: starting_player,
        current_player_index: starting_player,
        log: Vec::new(),
        board_history: Default::default(),
        log_index: 0,
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
        custom_ui_elements: HashMap::new(),
    };
    for i in 0..game.players.len() {
        ability::init_player(&mut game, i, all);
    }

    initialize_setup_log(&mut game);
    if game.civilization_draft.is_none() {
        execute_setup_round(setup.player_amount, &mut game, map_setup.as_ref());
        if setup.options.civilization == CivSetupOption::Random {
            game.next_age();
        }
    } else {
        // Draft choices precede the per-player setup grants, but undo locking
        // still needs a turn to anchor its cursor before any choice is made.
        add_turn_log(
            &mut game,
            TurnType::Setup(SetupTurnType {
                player: starting_player,
                civilization: None,
            }),
        );
        game.information_revealed();
    }
    game
}

fn initialize_setup_log(game: &mut Game) {
    let mut age = ActionLogAge::new(0);
    age.rounds.push(ActionLogRound::new(0));
    game.log.push(age);
}

fn execute_setup_round(player_amount: usize, game: &mut Game, map_setup: Option<&MapSetup>) {
    let choose_civ = game.options.civilization == CivSetupOption::ChooseCivilization;
    for player_index in 0..player_amount {
        add_turn_log(
            game,
            TurnType::Setup(SetupTurnType {
                player: player_index,
                civilization: if choose_civ {
                    None
                } else {
                    Some(game.player(player_index).civilization.name.clone())
                },
            }),
        );
        add_start_turn_action_if_needed(game, player_index);
        let origin = setup_event_origin();
        let player = &EventPlayer::new(player_index, origin.clone());
        player.gain_resources(game, ResourcePile::food(2));
        do_advance(game, Advance::Farming, player, false);
        do_advance(game, Advance::Mining, player, false);

        gain_action_card_from_pile(game, player);
        gain_objective_card_from_pile(game, player);
        if let Some(m) = &map_setup {
            let h = &m.home_positions[player_index];
            place_home_tiles(game, player);
            let position = h.block.tiles(&h.position, h.position.rotation)[0].0;
            gain_city(game, player, City::new(player_index, position));
            set_city_mood(game, position, &origin, MoodState::Happy);
            gain_unit(game, player, position, UnitType::Settler);
        }
    }
}

pub(crate) fn place_home_tiles(game: &mut Game, player: &EventPlayer) {
    let h = &get_map_setup(game.players.iter().filter(|p| p.is_human()).count()).home_positions
        [player.index];
    let home = player
        .get(game)
        .civilization
        .start_block
        .as_ref()
        .unwrap_or(&h.block)
        .clone();
    game.map
        .add_block_tiles(&h.position, &home, h.position.rotation);
}

#[must_use]
pub fn setup_event_origin() -> EventOrigin {
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
    let mut civilizations = cache.get_civilizations().clone();
    for player_index in 0..setup.player_amount {
        let civilization = player_civ(setup, rng, cache, &mut civilizations, player_index);
        let mut player = Player::new(civilization, player_index);
        player.resource_limit = ResourcePile::new(2, 7, 7, 7, 7, 0, 0);
        player.incident_tokens = 3;
        players.push(player);
    }
    players
}

fn player_civ(
    setup: &GameSetup,
    rng: &mut Rng,
    cache: &Cache,
    civilizations: &mut Vec<Civilization>,
    player_index: usize,
) -> Civilization {
    let random = setup.options.civilization == CivSetupOption::Random;
    if setup.assigned_civilizations.is_empty() {
        if random {
            civilizations.remove(rng.range(0, civilizations.len()))
        } else {
            cache.get_civilization(CHOOSE_CIV)
        }
    } else {
        if random {
            rng.next_seed(); // need to call next_seed to have the same number of calls to rng
        }
        cache.get_civilization(&setup.assigned_civilizations[player_index])
    }
}

pub(crate) fn execute_choose_civ(
    game: &mut Game,
    player_index: usize,
    action: &Action,
) -> Result<(), String> {
    let player = EventPlayer::new(player_index, setup_event_origin());
    if let Action::ChooseCivilization(civ) = action {
        let civilization = game
            .cache
            .get_civilizations()
            .iter()
            .find(|c| &c.name == civ)
            .ok_or("Choose a playable civilization")?
            .clone();
        if civilization.is_used(game).is_some() {
            return Err("Civilization already chosen".into());
        }
        game.player_mut(player_index).civilization = civilization;
        let p = game.player_mut(player_index);
        p.available_leaders = all_leaders(&p.civilization);
        place_home_tiles(game, &player);
    } else {
        return Err("action should be a choose civ action".to_string());
    }

    game.increment_player_index();
    if game.players.iter().all(|p| !p.civilization.is_choose_civ()) {
        game.state = GameState::Playing;
        game.next_age();
    }
    Ok(())
}

pub(crate) fn all_leaders(civilization: &Civilization) -> Vec<Leader> {
    civilization
        .leaders
        .iter()
        .map(|leader| leader.leader)
        .collect_vec()
}

/// Only store the private lock. No civilization-dependent data or log entries
/// are produced until the final lock, so even the first chooser stays hidden.
pub(crate) fn execute_draft_choice(
    game: &mut Game,
    player: usize,
    action: &Action,
) -> Result<(), String> {
    let draft = game
        .civilization_draft
        .as_mut()
        .ok_or("No civilization draft")?;
    let Action::ChooseCivilization(civilization) = action else {
        return Err("Choose one of your three civilizations".into());
    };
    if player >= draft.ready.len() || game.dropped_players.contains(&player) {
        return Err("This player cannot choose a civilization now".into());
    }
    if !draft.offers[player].contains(civilization) {
        return Err("Choose one of your three civilizations".into());
    }
    draft.live_update = draft.ready[player];
    draft.choices[player] = Some(civilization.clone());
    draft.ready[player] = true;
    finish_civilization_draft(game);
    Ok(())
}

fn finish_civilization_draft(game: &mut Game) {
    if !game
        .civilization_draft
        .as_ref()
        .is_some_and(|d| d.ready.iter().all(|r| *r))
    {
        return;
    }
    let draft = game.civilization_draft.take().expect("draft exists");
    for (i, choice) in draft.choices.into_iter().enumerate() {
        let civilization = game.cache.get_civilization(&choice.expect("locked choice"));
        let p = game.player_mut(i);
        p.available_leaders = all_leaders(&civilization);
        p.civilization = civilization;
    }
    let player_amount = draft.ready.len();
    let map_setup = draft.random_map.then(|| get_map_setup(player_amount));
    if let Some(setup) = &map_setup {
        game.map = Map::random_map(&mut game.rng, setup);
    }
    game.state = GameState::Playing;
    if game.dropped_players.contains(&game.starting_player_index) {
        game.starting_player_index = (0..player_amount)
            .find(|i| !game.dropped_players.contains(i))
            .unwrap_or(0);
    }
    execute_setup_round(player_amount, game, map_setup.as_ref());
    game.next_age();
    game.information_revealed();
}

pub(crate) fn drop_draft_player(game: &mut Game, player: usize) {
    let draft = game.civilization_draft.as_mut().expect("draft exists");
    if player >= draft.ready.len() || game.dropped_players.contains(&player) {
        return;
    }
    draft.live_update = false;
    if !draft.ready[player] {
        draft.choices[player] = draft.offers[player].first().cloned();
        draft.ready[player] = true;
    }
    game.dropped_players.push(player);
    if game.dropped_players.len() == draft.ready.len() {
        game.civilization_draft = None;
        game.state = GameState::Finished;
    } else {
        finish_civilization_draft(game);
    }
}

pub(crate) fn civilization_choices(game: &Game, player: usize) -> Vec<&Civilization> {
    if game.state != GameState::ChooseCivilization {
        return vec![];
    }
    game.cache
        .get_civilizations()
        .iter()
        .filter(|c| {
            if let Some(draft) = &game.civilization_draft {
                !game.dropped_players.contains(&player) && draft.offers[player].contains(&c.name)
            } else {
                player == game.active_player() && c.can_choose() && c.is_used(game).is_none()
            }
        })
        .collect()
}
