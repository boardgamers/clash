use server::content::custom_actions::CustomAction;
use server::content::custom_phase_actions::CustomPhaseEventAction;
use server::status_phase::{
    ChangeGovernment, ChangeGovernmentType, RazeSize1City, StatusPhaseAction,
};

use server::content::trade_routes::find_trade_routes;
use server::events::EventOrigin;
use server::unit::{MoveUnits, UnitType, Units};
use server::{
    action::Action,
    city::{City, MoodState::*},
    city_pieces::Building::*,
    content::custom_actions::CustomAction::*,
    game::Game,
    game_api,
    map::Terrain::*,
    playing_actions,
    playing_actions::PlayingAction::*,
    position::Position,
    resource_pile::ResourcePile,
    unit::MovementAction::*,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::MAIN_SEPARATOR as SEPARATOR,
    vec,
};

#[test]
fn basic_actions() {
    let seed = String::new();
    let mut game = Game::new(1, seed, false);
    game.wonders_left.retain(|w| w.name == "Pyramids");
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
        player
            .advances
            .iter()
            .map(|a| a.name.clone())
            .collect::<Vec<String>>()
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
        city_piece: Observatory,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        port_position: None,
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
            payment: ResourcePile::mood_tokens(2),
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
            .mood_modified_size(player)
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
    let recruit_action = Action::Playing(Recruit(server::playing_actions::Recruit {
        units: Units::new(1, 0, 0, 0, 0, 0),
        city_position,
        payment: ResourcePile::food(2),
        leader_name: None,
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
    Action::Movement(Move(MoveUnits {
        units,
        destination,
        embark_carrier_id: None,
        payment: ResourcePile::empty(),
    }))
}

fn assert_undo(
    game: &Game,
    can_undo: bool,
    can_redo: bool,
    action_log_len: usize,
    action_log_index: usize,
    undo_limit: usize,
) {
    assert_eq!(can_undo, game.can_undo(), "can_undo");
    assert_eq!(can_redo, game.can_redo(), "can_redo");
    assert_eq!(action_log_len, game.action_log.len(), "action_log_len");
    assert_eq!(action_log_index, game.action_log_index, "action_log_index");
    assert_eq!(undo_limit, game.undo_limit, "undo_limit");
}

fn increase_happiness(game: Game) -> Game {
    let increase_happiness_action =
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![(Position::new(0, 0), 1)],
            payment: ResourcePile::mood_tokens(1),
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
    let game = game_api::execute_action(game, Action::Redo, 0);
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
    if update_expected() {
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

type TestAssert = Vec<Box<dyn FnOnce(&Game)>>;

struct TestAction {
    action: Action,
    undoable: bool,
    illegal_action_test: bool,
    player_index: usize,
    pre_asserts: TestAssert,
    post_asserts: TestAssert,
}

impl TestAction {
    fn new(action: Action, undoable: bool, illegal_action_test: bool, player_index: usize) -> Self {
        Self {
            action,
            undoable,
            illegal_action_test,
            player_index,
            pre_asserts: vec![],
            post_asserts: vec![],
        }
    }
    fn illegal(player_index: usize, action: Action) -> Self {
        Self::new(action, false, true, player_index)
    }

    fn undoable(player_index: usize, action: Action) -> Self {
        Self::new(action, true, false, player_index)
    }

    fn not_undoable(player_index: usize, action: Action) -> Self {
        Self::new(action, false, false, player_index)
    }

    fn with_pre_assert(mut self, pre_assert: impl FnOnce(&Game) + 'static) -> Self {
        self.pre_asserts.push(Box::new(pre_assert));
        self
    }

    #[allow(dead_code)]
    fn with_post_assert(mut self, post_assert: impl FnOnce(&Game) + 'static) -> Self {
        self.post_asserts.push(Box::new(post_assert));
        self
    }
}

fn test_actions(name: &str, actions: Vec<TestAction>) {
    let outcome: fn(name: &str, i: usize) -> String = |name, i| {
        if i == 0 {
            format!("{name}.outcome")
        } else {
            format!("{name}.outcome{}", i)
        }
    };
    for (i, action) in actions.into_iter().enumerate() {
        let from = if i == 0 {
            name.to_string()
        } else {
            outcome(name, i - 1)
        };
        let err = catch_unwind(AssertUnwindSafe(|| {
            test_action_internal(&from, outcome(name, i).as_str(), action);
        }));
        assert!(err.is_ok(), "test action {} should not panic", i);
    }
}

fn test_action(
    name: &str,
    action: Action,
    player_index: usize,
    undoable: bool,
    illegal_action_test: bool,
) {
    let outcome = format!("{name}.outcome");
    test_action_internal(
        name,
        &outcome,
        TestAction::new(action, undoable, illegal_action_test, player_index),
    );
}

fn test_action_internal(name: &str, outcome: &str, test: TestAction) {
    let action = test.action;
    let a = serde_json::to_string(&action).expect("action should be serializable");
    let a2 = serde_json::from_str(&a).expect("action should be deserializable");
    let game = load_game(name);
    for pre_assert in test.pre_asserts {
        pre_assert(&game);
    }

    if test.illegal_action_test {
        assert_illegal_action(game, test.player_index, a2);
        return;
    }
    let game = game_api::execute_action(game, a2, test.player_index);
    let expected_game = read_game_str(outcome);
    assert_eq_game_json(
        &expected_game,
        &to_json(&game),
        name,
        outcome,
        &format!("EXECUTE: the game did not match the expectation after the initial {name} action"),
    );
    if !test.undoable {
        assert!(!game.can_undo(), "should not be able to undo");
        return;
    }
    for post_assert in test.post_asserts {
        post_assert(&game);
    }
    undo_redo(
        name,
        test.player_index,
        &read_game_str(name),
        game,
        outcome,
        &read_game_str(outcome),
        0,
    );
}

struct IllegalActionTest {
    fail: bool,
    setup_done: bool,
}

fn illegal_action_test(run: impl Fn(&mut IllegalActionTest)) {
    run(&mut IllegalActionTest {
        fail: false,
        setup_done: false,
    }); // should not panic
    let mut test = IllegalActionTest {
        fail: true,
        setup_done: false,
    };
    let err = catch_unwind(AssertUnwindSafe(|| {
        run(&mut test);
    }));
    assert!(
        test.setup_done,
        "illegal action test should run setup before panic"
    );
    assert!(err.is_err(), "illegal action test should panic");
}

fn assert_illegal_action(game: Game, player: usize, action: Action) {
    let err = catch_unwind(AssertUnwindSafe(|| {
        let _ = game_api::execute_action(game, action, player);
    }));
    assert!(err.is_err(), "execute action should panic");
}

fn to_json(game: &Game) -> String {
    serde_json::to_string_pretty(&game.cloned_data()).expect("game data should be serializable")
}

fn read_game_str(name: &str) -> String {
    let path = game_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("game file {path} should exist in the test games folder: {e}"))
}

fn undo_redo(
    name: &str,
    player_index: usize,
    original_game: &str,
    game: Game,
    outcome: &str,
    expected_game: &str,
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
fn test_movement_on_roads_from_city() {
    let units = vec![0];
    let destination = Position::from_offset("F7");
    test_action(
        "movement_on_roads_from_city",
        Action::Movement(Move(MoveUnits {
            units,
            destination,
            embark_carrier_id: None,
            payment: ResourcePile::food(1) + ResourcePile::ore(1),
        })),
        1,
        true,
        false,
    );
}

#[test]
fn test_movement_on_roads_to_city() {
    let units = vec![0];
    let destination = Position::from_offset("D8");
    test_action(
        "movement_on_roads_to_city",
        Action::Movement(Move(MoveUnits {
            units,
            destination,
            embark_carrier_id: None,
            payment: ResourcePile::food(1) + ResourcePile::ore(1),
        })),
        1,
        true,
        false,
    );
}

#[test]
fn test_road_coordinates() {
    let game = &load_game("roads_unit_test");
    // city units at D8 are 0, 1, 2

    // 3 and 4 are on mountain C8 and can move to the city at D8 (ignoring movement restrictions),
    // but not both, since the city already has 3 army units
    assert!(get_destinations(game, &[4], "C8").contains(&"D8".to_string()));
    assert!(!get_destinations(game, &[3, 4], "C8").contains(&"D8".to_string()));

    // 5 and 6 are on E8 and count against the stack size limit of the units moving out of city D8
    // so only 2 can move over them towards F7
    assert!(get_destinations(game, &[0, 1], "D8").contains(&"F7".to_string()));
    let city_dest = get_destinations(game, &[0, 1, 2], "D8");
    assert!(!city_dest.contains(&"F7".to_string()));

    // all 3 city units can move around the mountain to C7
    assert!(city_dest.contains(&"C7".to_string()));
    // explore for the city units at D6 is not allowed
    assert!(!city_dest.contains(&"D6".to_string()));
    // embark for the city units at E7 is not allowed
    assert!(!city_dest.contains(&"E7".to_string()));

    // don't move to same position
    assert!(!city_dest.contains(&"D8".to_string()));
}

fn get_destinations(game: &Game, units: &[u32], position: &str) -> Vec<String> {
    let player = game.get_player(1);
    player
        .move_units_destinations(game, units, Position::from_offset(position), None)
        .unwrap()
        .into_iter()
        .map(|r| r.destination.to_string())
        .collect()
}

#[test]
fn test_theaters() {
    test_action(
        "theaters",
        Action::Playing(Custom(Theaters(ResourcePile::culture_tokens(1)))),
        0,
        true,
        false,
    );
}

#[test]
fn test_taxes() {
    test_action(
        "taxes",
        Action::Playing(Custom(Taxes(ResourcePile::new(1, 1, 1, 0, 1, 0, 0)))),
        0,
        true,
        false,
    );
}

#[test]
fn test_trade_route_coordinates() {
    let game = &load_game("trade_routes_unit_test");
    // trading cities are C6, D6, E6

    // our units are at C8, but the path is not explored
    // 4 ships on E7 can trade with E6
    // settler on the ship can trade with D6

    let found = find_trade_routes(game, game.get_player(1));
    assert_eq!(found.len(), 3);
}

#[test]
fn test_trade_routes() {
    test_action("trade_routes", Action::Playing(EndTurn), 0, false, false);
}

#[test]
fn test_trade_routes_with_currency() {
    test_actions(
        "trade_routes_with_currency",
        vec![
            TestAction::not_undoable(0, Action::Playing(EndTurn)),
            TestAction::undoable(
                1,
                Action::CustomPhaseEvent(CustomPhaseEventAction::ResourceReward(
                    ResourcePile::gold(1) + ResourcePile::food(1),
                )),
            ),
        ],
    );
}

#[test]
fn test_cultural_influence_instant() {
    test_action(
        "cultural_influence_instant",
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
fn test_cultural_influence_instant_with_arts() {
    test_action(
        "cultural_influence_instant_with_arts",
        Action::Playing(Custom(ArtsInfluenceCultureAttempt(
            playing_actions::InfluenceCultureAttempt {
                starting_city_position: Position::from_offset("C1"),
                target_player_index: 0,
                target_city_position: Position::from_offset("C2"),
                city_piece: Fortress,
            },
        ))),
        1,
        false,
        false,
    );
}

#[test]
fn test_cultural_influence_with_conversion() {
    test_actions(
        "cultural_influence_with_conversion",
        vec![
            TestAction::not_undoable(1, influence_action()),
            TestAction::undoable(1, Action::CulturalInfluenceResolution(true)),
        ],
    );
}

#[test]
fn test_cultural_influence() {
    test_actions(
        "cultural_influence",
        vec![
            TestAction::not_undoable(1, influence_action()),
            TestAction::undoable(1, Action::CulturalInfluenceResolution(true)),
        ],
    );
}

#[test]
fn test_separation_of_power() {
    illegal_action_test(|test| {
        let mut game = load_game("cultural_influence");
        game.execute_action(Action::Playing(EndTurn), 1);
        if test.fail {
            game.execute_action(
                Action::Playing(Advance {
                    advance: String::from("Separation of Power"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game.execute_action(Action::Playing(EndTurn), 0);
        test.setup_done = true;
        game.execute_action(influence_action(), 1);
    });
}

#[test]
fn test_devotion() {
    illegal_action_test(|test| {
        let mut game = load_game("cultural_influence");
        game.execute_action(Action::Playing(EndTurn), 1);
        if test.fail {
            game.execute_action(
                Action::Playing(Advance {
                    advance: String::from("Devotion"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game.execute_action(Action::Playing(EndTurn), 0);
        test.setup_done = true;
        game.execute_action(influence_action(), 1);
    });
}

#[test]
fn test_totalitarianism() {
    illegal_action_test(|test| {
        let mut game = load_game("cultural_influence");
        game.execute_action(Action::Playing(EndTurn), 1);
        if test.fail {
            game.execute_action(
                Action::Playing(Advance {
                    advance: String::from("Totalitarianism"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game.execute_action(Action::Playing(EndTurn), 0);
        test.setup_done = true;
        game.execute_action(influence_action(), 1);
    });
}

#[test]
fn test_monuments() {
    illegal_action_test(|test| {
        let mut game = load_game("cultural_influence");
        game.execute_action(Action::Playing(EndTurn), 1);
        if test.fail {
            game.execute_action(
                Action::Playing(Advance {
                    advance: String::from("Monuments"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game.execute_action(
            Action::Playing(Custom(ConstructWonder {
                city_position: Position::from_offset("C2"),
                wonder: String::from("Pyramids"),
                payment: ResourcePile::new(2, 3, 3, 0, 0, 0, 4),
            })),
            0,
        );
        game.execute_action(Action::Playing(EndTurn), 0);
        test.setup_done = true;
        game.execute_action(influence_action(), 1);
    });
}

fn influence_action() -> Action {
    Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: Position::from_offset("B3"),
            target_player_index: 0,
            target_city_position: Position::from_offset("C2"),
            city_piece: Temple,
        },
    ))
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
fn test_increase_happiness() {
    test_action(
        "increase_happiness",
        Action::Playing(IncreaseHappiness(playing_actions::IncreaseHappiness {
            happiness_increases: vec![
                (Position::from_offset("C2"), 1),
                (Position::from_offset("B3"), 2),
            ],
            payment: ResourcePile::mood_tokens(5),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_increase_happiness_sports() {
    test_action(
        "increase_happiness_sports",
        Action::Playing(Custom(Sports {
            payment: ResourcePile::culture_tokens(1),
            city_position: Position::from_offset("C2"),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_increase_happiness_sports2() {
    test_action(
        "increase_happiness_sports2",
        Action::Playing(Custom(Sports {
            payment: ResourcePile::culture_tokens(2),
            city_position: Position::from_offset("C2"),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_increase_happiness_voting() {
    test_action(
        "increase_happiness_voting",
        Action::Playing(Custom(VotingIncreaseHappiness(
            playing_actions::IncreaseHappiness {
                happiness_increases: vec![
                    (Position::from_offset("C2"), 1),
                    (Position::from_offset("B3"), 2),
                ],
                payment: ResourcePile::mood_tokens(5),
            },
        ))),
        0,
        true,
        false,
    );
}

#[test]
fn test_increase_happiness_voting_rituals() {
    test_action(
        "increase_happiness_voting_rituals",
        Action::Playing(Custom(CustomAction::VotingIncreaseHappiness(
            playing_actions::IncreaseHappiness {
                happiness_increases: vec![
                    (Position::from_offset("C2"), 1),
                    (Position::from_offset("B3"), 2),
                ],
                payment: ResourcePile::new(1, 0, 1, 1, 1, 1, 0),
            },
        ))),
        0,
        true,
        false,
    );
}

#[test]
fn test_absolute_power() {
    test_action(
        "absolute_power",
        Action::Playing(Custom(AbsolutePower)),
        0,
        true,
        false,
    );
}

#[test]
fn test_forced_labor() {
    test_actions(
        "forced_labor",
        vec![
            TestAction::undoable(0, Action::Playing(Custom(ForcedLabor))),
            TestAction::undoable(
                0,
                Action::Playing(Collect(playing_actions::Collect {
                    city_position: Position::from_offset("A1"),
                    collections: vec![
                        (Position::from_offset("A1"), ResourcePile::food(1)),
                        (Position::from_offset("A2"), ResourcePile::wood(1)),
                    ],
                })),
            ),
        ],
    );
}

#[test]
fn test_civil_liberties() {
    test_actions(
        "civil_liberties",
        vec![
            TestAction::undoable(0, Action::Playing(Custom(CivilRights))),
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::playing_actions::Recruit {
                    units: Units::new(0, 1, 0, 0, 0, 0),
                    city_position: Position::from_offset("A1"),
                    payment: ResourcePile::mood_tokens(2),
                    leader_name: None,
                    replaced_units: vec![],
                })),
            ),
        ],
    );
}

#[test]
fn test_recruit() {
    test_action(
        "recruit",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: Units::new(1, 1, 0, 0, 0, 0),
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::gold(2),
            leader_name: None,
            replaced_units: vec![4],
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_overpay() {
    test_action(
        "sanitation_and_draft",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: Units::new(0, 1, 0, 0, 0, 0),
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::mood_tokens(1) + ResourcePile::gold(2), //paid too much
            leader_name: None,
            replaced_units: vec![],
        })),
        0,
        true,
        true,
    );
}

#[test]
fn test_sanitation_and_draft() {
    // we should figure out that sanitation or draft are used, but not both
    let units = Units::new(1, 1, 0, 0, 0, 0);
    let city_position = Position::from_offset("A1");
    test_actions(
        "sanitation_and_draft",
        vec![TestAction::undoable(
            0,
            Action::Playing(Recruit(server::playing_actions::Recruit {
                units: units.clone(),
                city_position,
                payment: ResourcePile::mood_tokens(1) + ResourcePile::gold(2),
                leader_name: None,
                replaced_units: vec![],
            })),
        )
        .with_pre_assert(move |game| {
            let options = game.players[0]
                .recruit_cost_without_replaced(&units, city_position, None, None)
                .unwrap()
                .cost;
            assert_eq!(3, options.conversions.len());
            assert_eq!(ResourcePile::mood_tokens(1), options.conversions[0].to);
            assert_eq!(ResourcePile::mood_tokens(1), options.conversions[1].to);
            assert_eq!(
                vec![
                    EventOrigin::Advance("Sanitation".to_string()),
                    EventOrigin::Advance("Draft".to_string())
                ],
                options.modifiers
            );
        })],
    );
}

#[test]
fn test_recruit_leader() {
    test_action(
        "recruit_leader",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: Units::new(0, 0, 0, 0, 0, 1),
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
            leader_name: Some("Alexander".to_string()),
            replaced_units: vec![],
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_replace_leader() {
    test_action(
        "replace_leader",
        Action::Playing(Recruit(server::playing_actions::Recruit {
            units: Units::new(0, 0, 0, 0, 0, 1),
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
            leader_name: Some("Kleopatra".to_string()),
            replaced_units: vec![10],
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_recruit_combat() {
    test_actions(
        "recruit_combat",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::playing_actions::Recruit {
                    units: Units::new(0, 0, 1, 0, 0, 0),
                    city_position: Position::from_offset("C2"),
                    payment: ResourcePile::wood(1) + ResourcePile::gold(1),
                    leader_name: None,
                    replaced_units: vec![],
                })),
            ),
            TestAction::undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::ResourceReward(
                    ResourcePile::mood_tokens(1),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::ResourceReward(
                    ResourcePile::gold(1),
                )),
            ),
        ],
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
fn test_collect_husbandry() {
    let action = Action::Playing(Collect(playing_actions::Collect {
        city_position: Position::from_offset("B3"),
        collections: vec![(Position::from_offset("B5"), ResourcePile::food(1))],
    }));
    test_actions(
        "collect_husbandry",
        vec![
            TestAction::undoable(0, action.clone()),
            TestAction::illegal(0, action.clone()), // illegal because it can't be done again
        ],
    );
}

#[test]
fn test_collect_free_economy() {
    test_action(
        "collect_free_economy",
        Action::Playing(Custom(CustomAction::FreeEconomyCollect(
            playing_actions::Collect {
                city_position: Position::from_offset("C2"),
                collections: vec![
                    (Position::from_offset("B1"), ResourcePile::ore(1)),
                    (Position::from_offset("B2"), ResourcePile::ore(1)),
                ],
            },
        ))),
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
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_free_education() {
    test_actions(
        "free_education",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Draft"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
            ),
            TestAction::undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(vec![
                    ResourcePile::ideas(1),
                ])),
            ),
        ],
    );
}

#[test]
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
fn test_barbarians_spawn() {
    test_actions(
        "barbarians_spawn",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("B3"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectUnitType(
                    UnitType::Elephant,
                )),
            ),
        ],
    );
}

#[test]
fn test_barbarians_move() {
    test_actions(
        "barbarians_move",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("B3"),
                )),
            ),
        ],
    );
}

#[test]
fn test_barbarians_attack() {
    test_actions(
        "barbarians_attack",
        vec![TestAction::not_undoable(
            0,
            Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
        )],
    );
}

#[test]
fn test_barbarians_recapture_city() {
    test_actions(
        "barbarians_recapture_city",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![5,6,7,8], Position::from_offset("C2")),
        )],
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
                additional_advances: vec![String::from("Devotion")],
            }),
        )),
        0,
        false,
        false,
    );
}

// combat

#[test]
fn test_remove_casualties_attacker() {
    test_actions(
        "remove_casualties_attacker",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectUnits(vec![0, 1])),
            ),
        ],
    );
}

#[test]
fn test_remove_casualties_defender() {
    test_actions(
        "remove_casualties_defender",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_metallurgy() {
    test_action(
        "direct_capture_city_metallurgy",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_direct_capture_city_fortress() {
    test_action(
        "direct_capture_city_fortress",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_direct_capture_city_only_fortress() {
    test_action(
        "direct_capture_city_only_fortress",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_combat_all_modifiers() {
    test_actions(
        "combat_all_modifiers",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(vec![ResourcePile::ore(
                    1,
                )])),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(vec![
                    ResourcePile::empty(),
                    ResourcePile::ore(2),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(vec![ResourcePile::ore(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_combat_fanaticism() {
    test_actions(
        "combat_fanaticism",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_retreat() {
    test_actions(
        "retreat",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Bool(true)),
            ),
        ],
    );
}

#[test]
fn test_do_not_retreat() {
    test_actions(
        "retreat_no",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Bool(false)),
            ),
        ],
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
        Action::Movement(Move(MoveUnits {
            units: vec![3, 4],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(8),
            payment: ResourcePile::empty(),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_embark_continue() {
    test_action(
        "ship_embark_continue",
        Action::Movement(Move(MoveUnits {
            units: vec![5, 6],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(9),
            payment: ResourcePile::empty(),
        })),
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

#[test]
fn test_ship_explore_move_not_possible() {
    test_action(
        "ship_explore_move_not_possible",
        Action::ExploreResolution(3),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_navigate() {
    test_action(
        "ship_navigate",
        move_action(vec![1], Position::from_offset("A7")),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_navigate_coordinates() {
    let mut game = load_game("ship_navigation_unit_test");

    let pairs = [
        ("B3", "B5"),
        ("B5", "A7"),
        ("A7", "F7"),
        ("G7", "G3"),
        ("G3", "B3"),
    ];

    for pair in pairs {
        let from = Position::from_offset(pair.0);
        let to = Position::from_offset(pair.1);
        assert_navigate(&mut game, from, to);
        assert_navigate(&mut game, to, from);
    }
}

fn assert_navigate(game: &mut Game, from: Position, to: Position) {
    game.players[1].get_unit_mut(1).unwrap().position = from;
    let result = game
        .get_player(1)
        .move_units_destinations(game, &[1], from, None)
        .is_ok_and(|d| d.iter().any(|route| route.destination == to));
    assert!(
        result,
        "expected to be able to move from {} to {}",
        from, to,
    );
}

#[test]
fn test_ship_combat() {
    test_actions(
        "ship_combat",
        vec![
            TestAction::not_undoable(0, move_action(vec![7, 8], Position::from_offset("D2"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectUnits(vec![1])),
            ),
        ],
    );
}

#[test]
fn test_ship_combat_war_ships() {
    test_action(
        "ship_combat_war_ships",
        move_action(vec![7, 8], Position::from_offset("D2")),
        0,
        false,
        false,
    );
}

fn load_game(name: &str) -> Game {
    let game = Game::from_data(
        serde_json::from_str(&read_game_str(name)).unwrap_or_else(|e| {
            panic!(
                "the game file should be deserializable {}: {}",
                game_path(name),
                e
            )
        }),
    );
    if update_expected() {
        write_result(&to_json(&game), &game_path(name));
    }
    game
}

fn update_expected() -> bool {
    env::var("UPDATE_EXPECTED")
        .ok()
        .is_some_and(|s| s == "true")
}

#[test]
fn test_ship_navigate_explore_move() {
    test_action(
        "ship_navigate_explore_move",
        move_action(vec![1], Position::from_offset("F2")),
        1,
        false,
        false,
    );
}

#[test]
fn test_ship_navigate_explore_not_move() {
    test_action(
        "ship_navigate_explore_not_move",
        move_action(vec![1], Position::from_offset("F2")),
        1,
        false,
        false,
    );
}

#[test]
fn test_civ_maya_leader_pakal() {
    test_actions(
        "civ_maya_leader_pakal",
        vec![
            TestAction::not_undoable(0, move_action(vec![10], Position::from_offset("B1"))),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("B2"),
                )),
            ),
        ],
    );
}

#[test]
fn test_dogma() {
    test_actions(
        "dogma",
        vec![
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Dogma"),
                    payment: ResourcePile::ideas(2),
                }),
            ),
            TestAction::undoable(
                1,
                Action::Playing(Construct(playing_actions::Construct {
                    city_position: Position::from_offset("C1"),
                    city_piece: Temple,
                    payment: ResourcePile::new(0, 1, 1, 0, 0, 0, 0),
                    port_position: None,
                })),
            ),
            TestAction::undoable(
                1,
                Action::CustomPhaseEvent(CustomPhaseEventAction::ResourceReward(
                    ResourcePile::culture_tokens(1),
                )),
            ),
            TestAction::undoable(
                1,
                Action::CustomPhaseEvent(CustomPhaseEventAction::AdvanceReward(
                    "Fanaticism".to_string(),
                )),
            ),
        ],
    );
}

#[test]
fn test_priesthood() {
    test_actions(
        "priesthood",
        vec![
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Math"),
                    payment: ResourcePile::empty(),
                }),
            ),
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Astronomy"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::illegal(
                1,
                Action::Playing(Advance {
                    advance: String::from("Astronomy"),
                    payment: ResourcePile::empty(),
                }),
            ),
        ],
    );
}
