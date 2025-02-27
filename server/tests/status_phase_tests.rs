use crate::common::test_action;
use server::action::Action;
use server::position::Position;
use server::status_phase::{
    ChangeGovernment, ChangeGovernmentType, RazeSize1City, StatusPhaseAction,
};

mod common;

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
