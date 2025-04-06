use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::card::{HandCard, validate_card_selection};
use server::content::persistent_events::{EventResponse, PersistentEventRequest};
use server::playing_actions;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("objective_cards", "instant");

#[test]
fn test_draft() {
    let r = Action::Playing(PlayingAction::Recruit(playing_actions::Recruit::new(
        &Units::new(0, 1, 0, 0, 0, 0),
        Position::from_offset("A1"),
        ResourcePile::mood_tokens(1),
    )));
    JSON.test(
        "draft",
        vec![
            TestAction::undoable(0, r.clone()).without_json_comparison(),
            TestAction::undoable(0, r).without_json_comparison(),
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
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(2),
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
                .without_json_comparison(),
            TestAction::not_undoable(0, move_action(vec![2, 3], Position::from_offset("B1")))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(3),
                ])),
            )
            .with_pre_assert(|game| {
                let PersistentEventRequest::SelectHandCards(c) = &game
                    .events
                    .last()
                    .expect("last event")
                    .player
                    .handler
                    .as_ref()
                    .expect("handler")
                    .request
                else {
                    panic!("Expected SelectHandCards request");
                };
                //can't fulfill both objectives with same name
                assert_eq!(c.choices.len(), 2);
                assert!(validate_card_selection(&c.choices, game).is_err());
            }),
        ],
    );
}
