#![allow(dead_code)]

use server::action::Action;
use server::advance::AdvanceAction;
use server::cache::Cache;
use server::city_pieces::Building::Temple;
use server::content::persistent_events::{EventResponse, SelectedStructure, Structure};
use server::game::{Game, GameContext};
use server::log::current_player_turn_log_mut;
use server::movement::MoveUnits;
use server::movement::MovementAction::Move;
use server::playing_actions::PlayingAction::{Advance, InfluenceCultureAttempt};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::{advance, cultural_influence, game_api};
use std::fmt::Display;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    mem,
    path::MAIN_SEPARATOR as SEPARATOR,
    vec,
};
use server::content::custom_actions::{CustomAction, CustomActionType};

#[derive(Clone)]
pub struct GamePath {
    directory: String,
    name: String,
}

impl GamePath {
    #[must_use]
    pub fn new(directory: &str, name: &str) -> Self {
        Self {
            directory: directory.to_string(),
            name: name.to_string(),
        }
    }

    #[must_use]
    pub fn path(&self) -> String {
        format!("{}{SEPARATOR}{}.json", self.directory, self.name)
    }
}

impl Display for GamePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path())
    }
}

pub struct JsonTest {
    pub parent: &'static str,
    pub directory: &'static str,
}

impl JsonTest {
    pub const fn new(directory: &'static str) -> Self {
        Self {
            parent: "",
            directory,
        }
    }

    pub const fn child(parent: &'static str, directory: &'static str) -> Self {
        Self { parent, directory }
    }

    pub fn test(&self, name: &str, actions: Vec<TestAction>) {
        test_actions(self, name, actions);
    }

    pub fn load_game(&self, name: &str) -> Game {
        load_game(&self.path(name))
    }

    fn dir(&self) -> String {
        let local = if self.parent.is_empty() {
            self.directory.to_string()
        } else {
            format!("{}{}{}", self.parent, SEPARATOR, self.directory)
        };
        format!("tests{SEPARATOR}test_games{SEPARATOR}{local}")
    }

    pub fn compare_game(&self, name: &str, game: &Game) {
        let path = self.path(name);
        assert_eq_game_json(
            &read_game_str(&path),
            &to_json(game),
            name,
            &path,
            "NEW: the game did not match",
        );
    }

    pub fn path(&self, name: &str) -> GamePath {
        GamePath::new(&self.dir(), name)
    }
}

fn assert_eq_game_json(
    expected: &str,
    actual: &str,
    name: &str,
    expected_path: &GamePath,
    message: &str,
) {
    let result_path = GamePath::new(
        &expected_path.directory,
        &format!("{}.result", expected_path.name),
    );
    if clean_json(expected) == clean_json(actual) {
        fs::remove_file(result_path.to_string()).unwrap_or(());
        return;
    }
    if update_expected() {
        write_result(actual, expected_path);
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

pub(crate) fn write_result(actual: &str, result_path: &GamePath) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(result_path.to_string())
        .expect("Failed to create output file");
    file.write_all(actual.as_bytes())
        .expect("Failed to write output file");
}

pub(crate) type TestAssert = Vec<Box<dyn FnOnce(Game)>>;

pub struct TestAction {
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

    pub fn with_pre_assert(mut self, pre_assert: impl FnOnce(Game) + 'static) -> Self {
        self.pre_asserts.push(Box::new(pre_assert));
        self
    }

    pub fn with_post_assert(mut self, post_assert: impl FnOnce(Game) + 'static) -> Self {
        self.post_asserts.push(Box::new(post_assert));
        self
    }

    pub fn skip_json(mut self) -> Self {
        self.compare_json = false;
        self
    }
}

struct TestContext {
    name: String,
    index: usize,
    phase: String,
}

impl TestContext {
    fn new(name: &str, index: usize, phase: &str) -> Self {
        Self {
            name: name.to_string(),
            index,
            phase: phase.to_string(),
        }
    }
}

impl Display for TestContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, index: {}, phase: {}",
            self.name, self.index, self.phase
        )
    }
}

pub(crate) fn test_actions(test: &JsonTest, name: &str, actions: Vec<TestAction>) {
    let outcome: fn(name: &GamePath, i: usize) -> GamePath = |name, i| {
        let local = if i == 0 {
            format!("{}.outcome", name.name)
        } else {
            format!("{}.outcome{}", name.name, i)
        };
        GamePath::new(&name.directory, &local)
    };
    let path = test.path(name);
    let mut game = load_game(&path);
    let mut last_json_compare = false;
    for (i, action) in actions.into_iter().enumerate() {
        let from = if i == 0 {
            path.clone()
        } else {
            outcome(&path, i - 1)
        };
        let compare = mem::replace(&mut last_json_compare, action.compare_json);
        let mut context = TestContext::new(name, i, "EXECUTE");
        game = catch_unwind(AssertUnwindSafe(|| {
            test_action_internal(
                game,
                name,
                &from,
                &outcome(&path, i),
                action,
                compare,
                &mut context,
            )
        }))
        .unwrap_or_else(|e| panic!("{context}: {e:?}"));
    }
}

fn test_action_internal(
    game: Game,
    name: &str,
    original: &GamePath,
    outcome: &GamePath,
    test: TestAction,
    last_json_compare: bool,
    context: &mut TestContext,
) -> Game {
    let action = test.action;
    let a = serde_json::to_string(&action).expect("action should be serializable");
    let a2 = serde_json::from_str(&a).expect("action should be deserializable");
    for pre_assert in test.pre_asserts {
        pre_assert(game.clone());
    }

    if test.illegal_action_test {
        assert_illegal_action(game.clone(), test.player_index, a2);
        return game;
    }
    let game = game_api::execute(game, a2, test.player_index);
    if test.compare_json {
        assert_eq_game_json(
            &read_game_str(outcome),
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
        post_assert(game.clone());
    }
    let compare_json = test.compare_json && last_json_compare;
    undo_redo(
        name,
        test.player_index,
        compare_json,
        if compare_json {
            read_game_str(original)
        } else {
            "".to_string()
        },
        game.clone(),
        original,
        outcome,
        if test.compare_json {
            read_game_str(outcome)
        } else {
            "".to_string()
        },
        context,
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

pub(crate) fn to_json(game: &Game) -> String {
    serde_json::to_string_pretty(&game.cloned_data()).expect("game data should be serializable")
}

fn read_game_str(name: &GamePath) -> String {
    fs::read_to_string(name.to_string())
        .unwrap_or_else(|e| panic!("game file {name} should exist in the test games folder: {e}"))
}

#[allow(clippy::too_many_arguments)]
fn undo_redo(
    name: &str,
    player_index: usize,
    compare_json: bool,
    original_game: String,
    game: Game,
    original_path: &GamePath,
    outcome_path: &GamePath,
    expected_game: String,
    context: &mut TestContext,
) {
    context.phase = "UNDO".to_string();
    let game = game_api::execute(game, Action::Undo, player_index);
    if compare_json {
        let mut trimmed_game = game.clone();
        current_player_turn_log_mut(&mut trimmed_game).items.pop();
        assert_eq_game_json(
            &original_game,
            &to_json(&trimmed_game),
            name,
            original_path,
            &format!(
                "UNDO: the game did not match the expectation after undoing the {name} action"
            ),
        );
    }
    context.phase = "REDO".to_string();
    let game = game_api::execute(game, Action::Redo, player_index);
    if compare_json {
        assert_eq_game_json(
            &expected_game,
            &to_json(&game),
            name,
            outcome_path,
            &format!(
                "REDO: the game did not match the expectation after redoing the {name} action"
            ),
        );
    }
}

fn update_expected() -> bool {
    env::var("UPDATE_EXPECTED")
        .ok()
        .is_some_and(|s| s == "true")
}

pub fn load_game(path: &GamePath) -> Game {
    let game = Game::from_data(
        serde_json::from_str(&read_game_str(path))
            .unwrap_or_else(|e| panic!("the game file should be deserializable {path}: {e}",)),
        Cache::new(),
        GameContext::Play,
    );
    if update_expected() {
        write_result(&to_json(&game), path);
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

pub fn advance_action(advance: advance::Advance, payment: ResourcePile) -> Action {
    Action::Playing(Advance(AdvanceAction::new(advance, payment)))
}

pub fn custom_action(
    action: CustomActionType,
) -> Action {
    Action::Playing(PlayingAction::Custom(CustomAction::new(action, None)))
}

pub fn influence_action() -> Action {
    Action::Playing(InfluenceCultureAttempt(
        cultural_influence::InfluenceCultureAttempt::new(
            SelectedStructure::new(Position::from_offset("C2"), Structure::Building(Temple)),
            PlayingActionType::InfluenceCultureAttempt,
        ),
    ))
}

pub fn payment_response(
    payment: ResourcePile,
) -> Action {
    payment_response(payment)
}
