use crate::common::TestAction;
use common::JsonTest;
use server::action::Action;
use server::content::custom_phase_actions::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::status_phase::{ChangeGovernment, ChangeGovernmentType};

mod common;

const JSON: JsonTest = JsonTest::new("status_phase");

#[test]
fn test_end_game() {
    JSON.test(
        "end_game",
        vec![TestAction::not_undoable(
            0,
            Action::Playing(PlayingAction::EndTurn),
        )],
    );
}

#[test]
fn test_free_advance() {
    JSON.test(
        "free_advance",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance("Philosophy".to_string())),
            ),
        ],
    );
}

#[test]
fn test_wrong_status_phase_action() {
    JSON.test(
        "illegal_free_advance",
        vec![TestAction::illegal(
            0,
            Action::Response(EventResponse::SelectPositions(vec![])),
        )],
    );
}

#[test]
fn test_raze_city() {
    JSON.test(
        "raze_city",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "A1",
            )])),
        )],
    );
}

#[test]
fn test_raze_city_decline() {
    JSON.test(
        "raze_city_decline",
        vec![
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectPositions(vec![]))),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![]))),
        ],
    );
}

#[test]
fn test_determine_first_player() {
    JSON.test(
        "determine_first_player",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectPlayer(1)),
        )],
    );
}

#[test]
fn test_change_government() {
    JSON.test(
        "change_government",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::ChangeGovernmentType(
                ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                    new_government: String::from("Theocracy"),
                    additional_advances: vec![String::from("Devotion")],
                }),
            )),
        )],
    );
}
