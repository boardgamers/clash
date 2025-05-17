use crate::common::{TestAction, payment_response};
use common::JsonTest;
use server::action::Action;
use server::advance;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::ChangeGovernment;

mod common;

const JSON: JsonTest = JsonTest::new("status_phase");

#[test]
fn test_end_game() {
    JSON.test("end_game", vec![TestAction::not_undoable(
        0,
        Action::Playing(PlayingAction::EndTurn),
    )]);
}

#[test]
fn test_free_advance() {
    JSON.test("free_advance", vec![
        TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)),
        TestAction::not_undoable(
            1,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::Storage)),
        ),
        TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::Philosophy)),
        ),
    ]);
}

#[test]
fn test_wrong_status_phase_action() {
    JSON.test("illegal_free_advance", vec![TestAction::illegal(
        0,
        Action::Response(EventResponse::SelectPositions(vec![])),
    )]);
}

#[test]
fn test_raze_city() {
    JSON.test("raze_city", vec![TestAction::not_undoable(
        0,
        Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
            "A1",
        )])),
    )]);
}

#[test]
fn test_raze_city_decline() {
    JSON.test("raze_city_decline", vec![
        TestAction::not_undoable(0, Action::Response(EventResponse::SelectPositions(vec![]))),
        TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![]))),
    ]);
}

#[test]
fn test_determine_first_player() {
    JSON.test("determine_first_player", vec![TestAction::not_undoable(
        0,
        Action::Response(EventResponse::SelectPlayer(1)),
    )]);
}

#[test]
fn test_change_government() {
    JSON.test("change_government", vec![
        TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![])))
            .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1)),
        )
        .skip_json(),
        TestAction::not_undoable(
            0,
            Action::Response(EventResponse::ChangeGovernmentType(ChangeGovernment::new(
                String::from("Theocracy"),
                vec![advance::Advance::Devotion],
            ))),
        ),
    ]);
}

#[test]
fn test_keep_government() {
    JSON.test("keep_government", vec![
        TestAction::not_undoable(1, Action::Response(EventResponse::SelectPositions(vec![])))
            .skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::empty())),
    ]);
}
