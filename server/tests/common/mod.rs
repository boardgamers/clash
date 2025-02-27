#![allow(dead_code)]

use server::action::Action;
use server::city_pieces::Building::Temple;
use server::game::Game;
use server::playing_actions::PlayingAction::InfluenceCultureAttempt;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::MoveUnits;
use server::unit::MovementAction::Move;
use server::{game_api, playing_actions};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::MAIN_SEPARATOR as SEPARATOR,
    vec,
};

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

pub(crate) type TestAssert = Vec<Box<dyn FnOnce(&Game)>>;

pub(crate) struct TestAction {
    action: Action,
    undoable: bool,
    illegal_action_test: bool,
    player_index: usize,
    pre_asserts: TestAssert,
    post_asserts: TestAssert,
    compare_json: bool,
}

impl TestAction {
    pub fn new(
        action: Action,
        undoable: bool,
        illegal_action_test: bool,
        player_index: usize,
    ) -> Self {
        Self {
            action,
            undoable,
            illegal_action_test,
            player_index,
            pre_asserts: vec![],
            post_asserts: vec![],
            compare_json: true,
        }
    }
    pub fn illegal(player_index: usize, action: Action) -> Self {
        Self::new(action, false, true, player_index)
    }

    pub fn undoable(player_index: usize, action: Action) -> Self {
        Self::new(action, true, false, player_index)
    }

    pub fn not_undoable(player_index: usize, action: Action) -> Self {
        Self::new(action, false, false, player_index)
    }

    pub fn with_pre_assert(mut self, pre_assert: impl FnOnce(&Game) + 'static) -> Self {
        self.pre_asserts.push(Box::new(pre_assert));
        self
    }

    pub fn with_post_assert(mut self, post_assert: impl FnOnce(&Game) + 'static) -> Self {
        self.post_asserts.push(Box::new(post_assert));
        self
    }

    pub fn without_json_comparison(mut self) -> Self {
        self.compare_json = false;
        self
    }
}

pub(crate) fn test_actions(name: &str, actions: Vec<TestAction>) {
    let outcome: fn(name: &str, i: usize) -> String = |name, i| {
        if i == 0 {
            format!("{name}.outcome")
        } else {
            format!("{name}.outcome{}", i)
        }
    };
    let mut game = load_game(name);
    for (i, action) in actions.into_iter().enumerate() {
        let from = if i == 0 {
            name.to_string()
        } else {
            outcome(name, i - 1)
        };
        game = catch_unwind(AssertUnwindSafe(|| {
            test_action_internal(game, &from, outcome(name, i).as_str(), action)
        }))
        .unwrap_or_else(|e| panic!("test action {i} should not panic: {e:?}"))
    }
}

pub fn test_action(
    name: &str,
    action: Action,
    player_index: usize,
    undoable: bool,
    illegal_action_test: bool,
) {
    let outcome = format!("{name}.outcome");
    test_action_internal(
        load_game(name),
        name,
        &outcome,
        TestAction::new(action, undoable, illegal_action_test, player_index),
    );
}

fn test_action_internal(game: Game, name: &str, outcome: &str, test: TestAction) -> Game {
    let action = test.action;
    let a = serde_json::to_string(&action).expect("action should be serializable");
    let a2 = serde_json::from_str(&a).expect("action should be deserializable");
    for pre_assert in test.pre_asserts {
        pre_assert(&game);
    }

    if test.illegal_action_test {
        assert_illegal_action(game.clone(), test.player_index, a2);
        return game;
    }
    let game = game_api::execute(game, a2, test.player_index);
    if test.compare_json {
        let expected_game = read_game_str(outcome);
        assert_eq_game_json(
            &expected_game,
            &to_json(&game),
            name,
            outcome,
            &format!(
                "EXECUTE: the game did not match the expectation after the initial {name} action"
            ),
        );
    }
    if !test.undoable {
        assert!(!game.can_undo(), "should not be able to undo");
        return game;
    }
    for post_assert in test.post_asserts {
        post_assert(&game);
    }
    undo_redo(
        name,
        test.player_index,
        test.compare_json,
        if test.compare_json {
            read_game_str(name)
        } else {
            "".to_string()
        },
        game.clone(),
        outcome,
        if test.compare_json {
            read_game_str(outcome)
        } else {
            "".to_string()
        },
        0,
    );
    game
}

pub struct IllegalActionTest {
    pub fail: bool,
    pub setup_done: bool,
}

pub fn illegal_action_test(run: impl Fn(&mut IllegalActionTest)) {
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
        let _ = game_api::execute(game, action, player);
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

#[allow(clippy::too_many_arguments)]
fn undo_redo(
    name: &str,
    player_index: usize,
    compare_json: bool,
    original_game: String,
    game: Game,
    outcome: &str,
    expected_game: String,
    cycle: usize,
) {
    if cycle == 2 {
        return;
    }
    let game = game_api::execute(game, Action::Undo, player_index);
    if compare_json {
        let mut trimmed_game = game.clone();
        trimmed_game.action_log.pop();
        assert_eq_game_json(
            &original_game,
            &to_json(&trimmed_game),
            name,
            name,
            &format!(
                "UNDO {cycle}: the game did not match the expectation after undoing the {name} action"
            ),
        );
    }
    let game = game_api::execute(game, Action::Redo, player_index);
    if compare_json {
        assert_eq_game_json(
            &expected_game,
            &to_json(&game),
            name,
            outcome,
            &format!(
            "REDO {cycle}: the game did not match the expectation after redoing the {name} action"
        ),
        );
    }
    undo_redo(
        name,
        player_index,
        compare_json,
        original_game,
        game,
        outcome,
        expected_game,
        cycle + 1,
    );
}

fn update_expected() -> bool {
    env::var("UPDATE_EXPECTED")
        .ok()
        .is_some_and(|s| s == "true")
}

pub fn load_game(name: &str) -> Game {
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

pub fn move_action(units: Vec<u32>, destination: Position) -> Action {
    Action::Movement(Move(MoveUnits {
        units,
        destination,
        embark_carrier_id: None,
        payment: ResourcePile::empty(),
    }))
}

pub fn influence_action() -> Action {
    Action::Playing(InfluenceCultureAttempt(
        playing_actions::InfluenceCultureAttempt {
            starting_city_position: Position::from_offset("B3"),
            target_player_index: 0,
            target_city_position: Position::from_offset("C2"),
            city_piece: Temple,
        },
    ))
}
