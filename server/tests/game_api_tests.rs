use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::MAIN_SEPARATOR as SEPARATOR,
};

use server::action::CombatAction;
use server::game::{CulturalInfluenceResolution, GameState};
use server::playing_actions::PlayingAction;
use server::status_phase::{
    ChangeGovernment, ChangeGovernmentType, RazeSize1City, StatusPhaseAction,
};
use server::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::Building::{self, *},
    content::custom_actions::CustomAction::*,
    game::Game,
    game_api,
    map::Terrain::*,
    playing_actions,
    playing_actions::PlayingAction::*,
    position::Position,
    resource_pile::ResourcePile,
    unit::{MovementAction::*, UnitType::*},
};

#[test]
fn basic_actions() {
    let seed = String::new();
    let mut game = Game::new(1, seed, false);
    let founded_city_position = Position::new(0, 1);
    game.map.tiles = HashMap::from([(founded_city_position, Forest)]);
    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(2, game.actions_left);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::empty(),
    });
    let mut game = game_api::execute_action(game, advance_action, 0);
    let player = &game.players[0];

    assert_eq!(
        vec![
            String::from("Farming"),
            String::from("Mining"),
            String::from("Math"),
            String::from("Engineering")
        ],
        player.advances
    );
    assert_eq!(ResourcePile::culture_tokens(1), player.resources);
    assert_eq!(1, game.actions_left);

    game.players[0].gain_resources(ResourcePile::new(2, 4, 4, 0, 2, 2, 3));
    let city_position = Position::new(0, 0);
    game.players[0].cities.push(City::new(0, city_position));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 3)));
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 2)));

    let construct_action = Action::Playing(Construct(playing_actions::Construct {
        city_position,
        city_piece: Building::Observatory,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        port_position: None,
        temple_bonus: None,
    }));
    let game = game_api::execute_action(game, construct_action, 0);
    let player = &game.players[0];

    assert_eq!(
        Some(0),
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .pieces
            .observatory
    );
    assert_eq!(
        2,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .size()
    );
    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 2, 4), player.resources);
    assert_eq!(0, game.actions_left);

    let game = game_api::execute_action(game, Action::Playing(EndTurn), 0);

    assert_eq!(3, game.actions_left);
    assert_eq!(0, game.active_player());

    let increase_happiness_action =
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![(city_position, 1)],
        }));
    let game = game_api::execute_action(game, increase_happiness_action, 0);
    let player = &game.players[0];

    assert_eq!(ResourcePile::new(1, 3, 3, 0, 2, 0, 4), player.resources);
    assert!(matches!(
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .mood_state,
        Happy
    ));
    assert_eq!(2, game.actions_left);

    let construct_wonder_action = Action::Playing(Custom(ConstructWonder {
        city_position,
        wonder: String::from("Pyramids"),
        payment: ResourcePile::new(1, 3, 3, 0, 2, 0, 4),
    }));
    let mut game = game_api::execute_action(game, construct_wonder_action, 0);
    let player = &game.players[0];

    assert_eq!(10.0, player.victory_points(&game));
    assert_eq!(ResourcePile::empty(), player.resources);
    assert_eq!(vec![String::from("Pyramids")], player.wonders_build);
    assert_eq!(
        1,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .pieces
            .wonders
            .len()
    );
    assert_eq!(
        4,
        player
            .get_city(city_position)
            .expect("player should have a city at this position")
            .mood_modified_size()
    );
    assert_eq!(1, game.actions_left);

    let tile_position = Position::new(1, 0);
    game.map.tiles.insert(tile_position, Mountain);
    game.map.tiles.insert(city_position, Fertile);
    let collect_action = Action::Playing(Collect(playing_actions::Collect {
        city_position,
        collections: vec![(tile_position, ResourcePile::ore(1))],
    }));
    let game = game_api::execute_action(game, collect_action, 0);
    let player = &game.players[0];
    assert_eq!(ResourcePile::ore(1), player.resources);
    assert!(player
        .get_city(city_position)
        .expect("player should have a city at this position")
        .is_activated());
    assert_eq!(0, game.actions_left);
    let mut game = game_api::execute_action(game, Action::Playing(EndTurn), 0);
    let player = &mut game.players[0];
    player.gain_resources(ResourcePile::food(2));
    let recruit_action =
        Action::Playing(PlayingAction::Recruit(server::playing_actions::Recruit {
            units: vec![Settler],
            city_position,
            payment: ResourcePile::food(2),
            leader_index: None,
            replaced_units: Vec::new(),
        }));
    let mut game = game_api::execute_action(game, recruit_action, 0);
    let player = &mut game.players[0];
    assert_eq!(1, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(ResourcePile::ore(1), player.resources);
    assert!(player
        .get_city(city_position)
        .expect("The player should have a city at this position")
        .is_activated());

    let game = game_api::execute_action(game, Action::Playing(MoveUnits), 0);
    let movement_action = move_action(vec![0], founded_city_position);
    let game = game_api::execute_action(game, movement_action, 0);
    let game = game_api::execute_action(game, Action::Movement(Stop), 0);
    let player = &game.players[0];
    assert_eq!(founded_city_position, player.units[0].position);

    let found_city_action = Action::Playing(FoundCity { settler: 0 });
    let game = game_api::execute_action(game, found_city_action, 0);
    let player = &game.players[0];
    assert_eq!(0, player.units.len());
    assert_eq!(1, player.next_unit_id);
    assert_eq!(4, player.cities.len());
    assert_eq!(
        1,
        player
            .get_city(founded_city_position)
            .expect("The player should have the founded city")
            .size()
    );
}

fn move_action(units: Vec<u32>, destination: Position) -> Action {
    Action::Movement(Move {
        units,
        destination,
        embark_carrier_id: None,
    })
}

#[test]
fn cultural_influence() {
    let mut game = Game::new(2, String::new(), false);
    game.dice_roll_outcomes = vec![10, 6, 8, 4];
    game.set_player_index(0);
    game.players[0].gain_resources(ResourcePile::culture_tokens(4));
    game.players[1].gain_resources(ResourcePile::culture_tokens(1));
    let city0position = Position::new(0, 0);
    let city1position = Position::new(2, 0);
    assert_eq!(city0position.distance(city1position), 2);
    game.players[0].cities.push(City::new(0, city0position));
    game.players[1].cities.push(City::new(1, city1position));
    game.players[1].construct(Academy, city1position, None);
    let influence_action = Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: city0position,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Academy,
        },
    ));
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(
        game.state,
        GameState::CulturalInfluenceResolution(CulturalInfluenceResolution {
            roll_boost_cost: 2,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Academy
        })
    );
    let influence_resolution_decline_action = Action::CulturalInfluenceResolution(false);
    let game = game_api::execute_action(game, influence_resolution_decline_action, 0);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, GameState::Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: city0position,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Academy,
        },
    ));
    let game = game_api::execute_action(game, influence_action, 0);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, GameState::Playing);
    assert!(game.successful_cultural_influence);
    let game = game_api::execute_action(game, Action::Playing(EndTurn), 0);
    assert_eq!(game.active_player(), 1);
    let influence_action = Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: city1position,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Academy,
        },
    ));
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(game.players[1].cities[0].influenced());
    assert_eq!(game.state, GameState::Playing);
    assert!(!game.successful_cultural_influence);
    let influence_action = Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: city1position,
            target_player_index: 1,
            target_city_position: city1position,
            city_piece: Academy,
        },
    ));
    let game = game_api::execute_action(game, influence_action, 1);
    assert!(!game.players[1].cities[0].influenced());
    assert_eq!(game.state, GameState::Playing);
    assert!(game.successful_cultural_influence);
}

fn assert_undo(
    game: &Game,
    can_undo: bool,
    can_redo: bool,
    action_log_len: usize,
    action_log_index: usize,
    undo_limit: usize,
) {
    assert_eq!(can_undo, game.can_undo());
    assert_eq!(can_redo, game.can_redo());
    assert_eq!(action_log_len, game.action_log.len());
    assert_eq!(action_log_index, game.action_log_index);
    assert_eq!(undo_limit, game.undo_limit);
}

fn increase_happiness(game: Game) -> Game {
    let increase_happiness_action =
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![(Position::new(0, 0), 1)],
        }));
    game_api::execute_action(game, increase_happiness_action, 0)
}

#[test]
fn undo() {
    let mut game = Game::new(1, String::new(), false);
    game.players[0]
        .cities
        .push(City::new(0, Position::new(0, 0)));
    game.players[0].gain_resources(ResourcePile::mood_tokens(2));
    game.players[0].cities[0].decrease_mood_state();

    assert_undo(&game, false, false, 0, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);
    let game = increase_happiness(game);
    assert_undo(&game, true, false, 1, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = increase_happiness(game);
    assert_undo(&game, true, false, 2, 2, 0);
    assert_eq!(Happy, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);
    let game = increase_happiness(game);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Redo, 0);
    assert_undo(&game, true, false, 2, 2, 0);
    assert_eq!(Happy, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, true, true, 2, 1, 0);
    assert_eq!(Neutral, game.players[0].cities[0].mood_state);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 2, 0, 0);
    assert_eq!(Angry, game.players[0].cities[0].mood_state);

    let advance_action = Action::Playing(Advance {
        advance: String::from("Math"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    assert_undo(&game, true, false, 1, 1, 0);
    let game = game_api::execute_action(game, Action::Undo, 0);
    assert_undo(&game, false, true, 1, 0, 0);
    assert_eq!(2, game.players[0].advances.len());
    let advance_action = Action::Playing(Advance {
        advance: String::from("Engineering"),
        payment: ResourcePile::food(2),
    });
    let game = game_api::execute_action(game, advance_action, 0);
    assert_undo(&game, false, false, 1, 1, 1);
}

fn assert_eq_game_json(
    expected: &str,
    actual: &str,
    name: &str,
    expected_name: &str,
    message: &str,
) {
    let result_path = game_path(&format!("{name}.result"));
    if clean_json(expected) == clean_json(actual) {
        fs::remove_file(&result_path).unwrap_or(());
        return;
    }
    let expected_path = game_path(expected_name);
    if env::var("UPDATE_EXPECTED")
        .ok()
        .is_some_and(|s| s == "true")
    {
        write_result(actual, &expected_path);
        return;
    } else {
        write_result(actual, &result_path);
    }

    panic!(
        "{name} test failed:\n\
        {message}.\n\
        Expected game was not equal to the actual game.\n\
        See 'expected' at {expected_path} and 'actual' at {result_path}."
    );
}

fn clean_json(expected: &str) -> String {
    expected.replace([' ', '\t', '\n', '\r'], "")
}

fn write_result(actual: &str, result_path: &String) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(result_path)
        .expect("Failed to create output file");
    file.write_all(actual.as_bytes())
        .expect("Failed to write output file");
}

fn game_path(name: &str) -> String {
    format!("tests{SEPARATOR}test_games{SEPARATOR}{name}.json")
}

fn test_action(
    name: &str,
    action: Action,
    player_index: usize,
    undoable: bool,
    illegal_action_test: bool,
) {
    let a = serde_json::to_string(&action).expect("action should be serializable");
    let a2 = serde_json::from_str(&a).expect("action should be deserializable");
    let original_game = read_game(name);
    let game =
        Game::from_data(serde_json::from_str(&original_game).unwrap_or_else(|_| {
            panic!("the game file should be deserializable {}", game_path(name))
        }));
    let game = game_api::execute_action(game, a2, player_index);
    if illegal_action_test {
        println!(
            "execute action was successful but should have panicked because the action is illegal"
        );
        return;
    }
    let outcome = format!("{name}.outcome");
    let expected_game = read_game(&outcome);
    assert_eq_game_json(
        &expected_game,
        &to_json(&game),
        name,
        &outcome,
        &format!("EXECUTE: the game did not match the expectation after the initial {name} action"),
    );
    if !undoable {
        assert!(!game.can_undo());
        return;
    }
    undo_redo(
        name,
        player_index,
        &original_game,
        game,
        &outcome,
        &expected_game,
        0,
    );
}

fn to_json(game: &Game) -> String {
    serde_json::to_string_pretty(&game.cloned_data()).expect("game data should be serializable")
}

fn read_game(name: &str) -> String {
    let path = game_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("game file {path} should exist in the test games folder"))
}

fn undo_redo(
    name: &str,
    player_index: usize,
    original_game: &String,
    game: Game,
    outcome: &String,
    expected_game: &String,
    cycle: usize,
) {
    if cycle == 2 {
        return;
    }
    let game = game_api::execute_action(game, Action::Undo, player_index);
    let mut trimmed_game = game.clone();
    trimmed_game.action_log.pop();
    assert_eq_game_json(
        original_game,
        &to_json(&trimmed_game),
        name,
        name,
        &format!(
            "UNDO {cycle}: the game did not match the expectation after undoing the {name} action"
        ),
    );
    let game = game_api::execute_action(game, Action::Redo, player_index);
    assert_eq_game_json(
        expected_game,
        &to_json(&game),
        name,
        outcome,
        &format!(
            "REDO {cycle}: the game did not match the expectation after redoing the {name} action"
        ),
    );
    undo_redo(
        name,
        player_index,
        original_game,
        game,
        outcome,
        expected_game,
        cycle + 1,
    );
}

#[test]
fn test_movement() {
    test_action(
        "movement",
        move_action(vec![4], Position::from_offset("B3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_cultural_influence_attempt() {
    test_action(
        "cultural_influence_attempt",
        Action::Playing(InfluenceCultureAttempt(
            playing_actions::InfluenceCultureAttempt {
                starting_city_position: Position::from_offset("C1"),
                target_player_index: 0,
                target_city_position: Position::from_offset("C2"),
                city_piece: Fortress,
            },
        )),
        1,
        false,
        false,
    );
}

#[test]
fn test_cultural_influence_resolution() {
    test_action(
        "cultural_influence_resolution",
        Action::CulturalInfluenceResolution(true),
        1,
        true,
        false,
    );
}

#[test]
fn test_found_city() {
    test_action(
        "found_city",
        Action::Playing(FoundCity { settler: 4 }),
        0,
        true,
        false,
    );
}

#[test]
fn test_wonder() {
    test_action(
        "wonder",
        Action::Playing(Custom(ConstructWonder {
            city_position: Position::from_offset("A1"),
            wonder: String::from("Pyramids"),
            payment: ResourcePile::new(2, 3, 3, 0, 0, 0, 4),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_custom_action_forced_labor() {
    test_action(
        "custom_action_forced_labor",
        Action::Playing(Custom(ForcedLabor {})),
        0,
        true,
        false,
    );
}

#[test]
fn test_recruit() {
    test_action(
        "recruit",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: vec![Settler, Infantry],
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::gold(2),
            leader_index: None,
            replaced_units: vec![4],
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_recruit_combat() {
    test_action(
        "recruit_combat",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: vec![Ship],
            city_position: Position::from_offset("C2"),
            payment: ResourcePile::wood(2),
            leader_index: None,
            replaced_units: vec![],
        })),
        0,
        false,
        false,
    );
}

#[test]
fn test_collect() {
    test_action(
        "collect",
        Action::Playing(Collect(playing_actions::Collect {
            city_position: Position::from_offset("C2"),
            collections: vec![
                (Position::from_offset("B1"), ResourcePile::ore(1)),
                (Position::from_offset("B2"), ResourcePile::wood(1)),
            ],
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_construct() {
    test_action(
        "construct",
        Action::Playing(Construct(playing_actions::Construct {
            city_position: Position::from_offset("C2"),
            city_piece: Observatory,
            payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
            port_position: None,
            temple_bonus: None,
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_construct_port() {
    test_action(
        "construct_port",
        Action::Playing(Construct(playing_actions::Construct {
            city_position: Position::from_offset("A1"),
            city_piece: Port,
            payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
            port_position: Some(Position::from_offset("A2")),
            temple_bonus: None,
        })),
        0,
        true,
        false,
    );
}

#[test]
#[should_panic(expected = "Illegal action")]
fn test_wrong_status_phase_action() {
    test_action(
        "illegal_free_advance",
        Action::StatusPhase(StatusPhaseAction::RazeSize1City(RazeSize1City::None)),
        0,
        false,
        true,
    );
}

// status phase

#[test]
fn test_free_advance() {
    test_action(
        "free_advance",
        Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
        0,
        false,
        false,
    );
}

#[test]
fn test_raze_city() {
    test_action(
        "raze_city",
        Action::StatusPhase(StatusPhaseAction::RazeSize1City(RazeSize1City::Position(
            Position::from_offset("A1"),
        ))),
        0,
        false,
        false,
    );
}

#[test]
fn test_raze_city_decline() {
    test_action(
        "raze_city_decline",
        Action::StatusPhase(StatusPhaseAction::RazeSize1City(RazeSize1City::None)),
        0,
        false,
        false,
    );
}

#[test]
fn test_determine_first_player() {
    test_action(
        "determine_first_player",
        Action::StatusPhase(StatusPhaseAction::DetermineFirstPlayer(1)),
        0,
        false,
        false,
    );
}

#[test]
fn test_change_government() {
    test_action(
        "change_government",
        Action::StatusPhase(StatusPhaseAction::ChangeGovernmentType(
            ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                new_government: String::from("Theocracy"),
                additional_advances: vec![String::from("Theocracy 2")],
            }),
        )),
        0,
        false,
        false,
    );
}

// combat

#[test]
fn test_until_remove_casualties_attacker() {
    test_action(
        "until_remove_casualties_attacker",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_remove_casualties_attacker_and_capture_city() {
    test_action(
        "remove_casualties_attacker",
        Action::Combat(CombatAction::RemoveCasualties(vec![0, 1])),
        0,
        false,
        false,
    );
}

#[test]
fn test_until_remove_casualties_defender() {
    test_action(
        "until_remove_casualties_defender",
        move_action(vec![0], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_remove_casualties_defender_and_defender_wins() {
    test_action(
        "remove_casualties_defender",
        Action::Combat(CombatAction::RemoveCasualties(vec![0])),
        1,
        false,
        false,
    );
}

#[test]
fn test_direct_capture_city() {
    test_action(
        "direct_capture_city",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_first_combat_round_no_hits_attacker_may_retreat() {
    test_action(
        "first_combat_round_no_hits",
        move_action(vec![0], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_retreat() {
    test_action(
        "retreat",
        Action::Combat(CombatAction::Retreat(true)),
        0,
        false,
        false,
    );
}

#[test]
fn test_do_not_retreat_and_next_combat_round() {
    test_action(
        "dont_retreat",
        Action::Combat(CombatAction::Retreat(false)),
        0,
        false,
        false,
    );
}

#[test]
fn test_explore_choose() {
    test_action(
        "explore_choose",
        move_action(vec![0], Position::from_offset("C7")),
        1,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_no_walk_on_water() {
    test_action(
        "explore_auto_no_walk_on_water",
        move_action(vec![0], Position::from_offset("B2")),
        0,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_adjacent_water() {
    test_action(
        "explore_auto_adjacent_water",
        move_action(vec![0], Position::from_offset("C7")),
        0,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_water_outside() {
    test_action(
        "explore_auto_water_outside",
        move_action(vec![1], Position::from_offset("F5")),
        1,
        false,
        false,
    );
}

#[test]
fn test_explore_resolution() {
    test_action(
        "explore_resolution",
        Action::ExploreResolution(3),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_transport() {
    // undo capture empty city is broken
    test_action(
        "ship_transport",
        move_action(vec![7], Position::from_offset("D2")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_transport_same_sea() {
    // undo capture empty city is broken
    test_action(
        "ship_transport_same_sea",
        move_action(vec![7], Position::from_offset("C3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_embark() {
    test_action(
        "ship_embark",
        Action::Movement(Move {
            units: vec![3, 4],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(8),
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_embark_continue() {
    test_action(
        "ship_embark_continue",
        Action::Movement(Move {
            units: vec![5, 6],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(9),
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_disembark() {
    // undo capture empty city is broken
    test_action(
        "ship_disembark",
        move_action(vec![1, 2], Position::from_offset("B3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_disembark_capture_empty_city() {
    test_action(
        "ship_disembark_capture_empty_city",
        move_action(vec![1, 2], Position::from_offset("B2")),
        0,
        false,
        false,
    );
}

#[test]
fn test_ship_explore() {
    test_action(
        "ship_explore",
        move_action(vec![1], Position::from_offset("C5")),
        1,
        false,
        false,
    );
}

#[test]
fn test_ship_explore_teleport() {
    test_action(
        "ship_explore_teleport",
        move_action(vec![1], Position::from_offset("C4")),
        1,
        false,
        false,
    );
}
