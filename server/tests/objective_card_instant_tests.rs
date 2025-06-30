use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::recruit;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("objective_cards", "instant");

#[test]
fn test_draft() {
    let r = Action::Playing(PlayingAction::Recruit(recruit::Recruit::new(
        &Units::new(0, 1, 0, 0, 0, None),
        Position::from_offset("A1"),
        ResourcePile::mood_tokens(1),
    )));
    JSON.test(
        "draft",
        vec![
            TestAction::undoable(0, r.clone()).skip_json(),
            TestAction::undoable(0, r).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(1),
                ])),
            ),
        ],
    )
}

#[test]
fn test_conqueror() {
    JSON.test(
        "conqueror",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(2),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(5),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(6),
                ])),
            ),
        ],
    );
}

#[test]
fn test_defiance() {
    JSON.test(
        "defiance",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectHandCards(vec![])))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(7),
                ])),
            ),
        ],
    );
}

#[test]
fn test_warmonger() {
    JSON.test(
        "warmonger",
        vec![
            TestAction::not_undoable(0, move_action(vec![0, 1], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(0, move_action(vec![2, 3], Position::from_offset("B1")))
                .skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(3),
                ])),
            ),
        ],
    );
}

#[test]
fn test_scavenger() {
    JSON.test(
        "scavenger",
        vec![
            TestAction::not_undoable(0, move_action(vec![0, 1], Position::from_offset("E7")))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(21),
                ])),
            ),
        ],
    );
}
