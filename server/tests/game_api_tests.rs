use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::MAIN_SEPARATOR as SEPARATOR,
};

use server::action::CombatAction;
use server::status_phase::{ChangeGovernmentType, StatusPhaseAction};
use server::{
    action::Action,
    city_pieces::Building::*,
    content::custom_actions::CustomAction::*,
    game::Game,
    game_api,
    playing_actions::PlayingAction::*,
    position::Position,
    resource_pile::ResourcePile,
    unit::{MovementAction::*, UnitType::*},
};

fn assert_eq_game_json(
    expected: &str,
    actual: &str,
    test: &str,
    expected_path: &str,
    message: &str,
) {
    if expected.replace([' ', '\t', '\n', '\r'], "") == actual.replace([' ', '\t', '\n', '\r'], "")
    {
        return;
    }
    let file_path = format!("tests{SEPARATOR}test_games{SEPARATOR}{test}.result.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&file_path)
        .expect("Failed to create output file");
    file.write_all(actual.as_bytes())
        .expect("Failed to write output file");
    let expected_path = format!("tests{SEPARATOR}test_games{SEPARATOR}{expected_path}.json");

    assert_eq!(
        actual,
        expected,
        "{}",
        format_args!(
            "{test} test failed:\n\
            {message}.\n\
            Expected game was not equal to the actual game.\n\
            See 'expected' at {expected_path} and 'actual' at {file_path}."
        )
    );
}

fn test_action(
    game_path: &str,
    action: Action,
    player_index: usize,
    undoable: bool,
    illegal_action_test: bool,
) {
    let path = format!("tests{SEPARATOR}test_games{SEPARATOR}{game_path}.json");
    let original_game =
        fs::read_to_string(path).expect("game file should exist in the test games folder");
    let game = Game::from_data(
        serde_json::from_str(&original_game).expect("the game file should be deserializable"),
    );
    let game = game_api::execute_action(game, action, player_index);
    if illegal_action_test {
        println!(
            "execute action was successful but should have panicked because the action is illegal"
        );
        return;
    }
    let json = serde_json::to_string_pretty(&game.cloned_data())
        .expect("game data should be serializable");
    let expected_path = format!("tests{SEPARATOR}test_games{SEPARATOR}{game_path}.outcome.json");
    let expected_game =
        fs::read_to_string(expected_path).expect("outcome file should be deserializable");
    assert_eq_game_json(
        &expected_game,
        &json,
        game_path,
        &(game_path.to_string() + ".outcome"),
        &format!("the game did not match the expectation after the initial {game_path} action"),
    );
    if !undoable {
        assert!(!game.can_undo());
        return;
    }
    let game = game_api::execute_action(game, Action::Undo, player_index);
    let mut trimmed_game = game.clone();
    trimmed_game.action_log.pop();
    let json = serde_json::to_string_pretty(&trimmed_game.cloned_data())
        .expect("game data should be serializable");
    assert_eq_game_json(
        &original_game,
        &json,
        game_path,
        game_path,
        &format!("the game did not match the expectation after undoing the {game_path} action"),
    );
    let game = game_api::execute_action(game, Action::Redo, player_index);
    let json = serde_json::to_string_pretty(&game.cloned_data())
        .expect("game data should be serializable");
    assert_eq_game_json(
        &expected_game,
        &json,
        game_path,
        &(game_path.to_string() + ".outcome"),
        &format!("the game did not match the expectation after redoing the {game_path} action"),
    );
}

#[test]
fn test_movement() {
    test_action(
        "movement",
        Action::Movement(Move {
            units: vec![4],
            destination: Position::from_offset("B3"),
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_cultural_influence_attempt() {
    test_action(
        "cultural_influence_attempt",
        Action::Playing(InfluenceCultureAttempt {
            starting_city_position: Position::from_offset("C1"),
            target_player_index: 0,
            target_city_position: Position::from_offset("C2"),
            city_piece: Fortress,
        }),
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
            wonder: String::from("X"),
            payment: ResourcePile::new(2, 3, 3, 0, 0, 0, 4),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_recruit() {
    test_action(
        "recruit",
        Action::Playing(Recruit {
            units: vec![Settler, Infantry],
            city_position: Position::from_offset("A1"),
            payment: ResourcePile::food(1) + ResourcePile::ore(1) + ResourcePile::gold(2),
            leader_index: None,
            replaced_units: vec![4],
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_collect() {
    test_action(
        "collect",
        Action::Playing(Collect {
            city_position: Position::from_offset("C2"),
            collections: vec![
                (Position::from_offset("B1"), ResourcePile::ore(1)),
                (Position::from_offset("B2"), ResourcePile::wood(1)),
            ],
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_construct() {
    test_action(
        "construct",
        Action::Playing(Construct {
            city_position: Position::from_offset("C2"),
            city_piece: Observatory,
            payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
            port_position: None,
            temple_bonus: None,
        }),
        0,
        true,
        false,
    );
}

#[test]
fn test_construct_port() {
    test_action(
        "construct_port",
        Action::Playing(Construct {
            city_position: Position::from_offset("A1"),
            city_piece: Port,
            payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
            port_position: Some(Position::from_offset("A2")),
            temple_bonus: None,
        }),
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
        Action::StatusPhase(StatusPhaseAction::RaseSize1City(None)),
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
        Action::StatusPhase(StatusPhaseAction::RaseSize1City(Some(
            Position::from_offset("A1"),
        ))),
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
        Action::StatusPhase(StatusPhaseAction::ChangeGovernmentType(Some(
            ChangeGovernmentType {
                new_government: String::from("Theocracy"),
                additional_advances: vec![String::from("Theocracy 2")],
            },
        ))),
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
        Action::Movement(Move {
            units: vec![0, 1, 2, 3],
            destination: Position::from_offset("C1"),
        }),
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
