use crate::common::{JsonTest, TestAction};
use server::action::Action;
use server::card::{HandCard, validate_card_selection};
use server::content::persistent_events::{EventResponse, PersistentEventRequest};
use server::playing_actions::PlayingAction;

mod common;

const JSON: JsonTest = JsonTest::child("objective_cards", "status_phase");

#[test]
fn test_large_civ() {
    JSON.test(
        "large_civ",
        vec![
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(1),
                    HandCard::ObjectiveCard(4),
                    HandCard::ObjectiveCard(5),
                    HandCard::ObjectiveCard(6),
                    HandCard::ObjectiveCard(7),
                    HandCard::ObjectiveCard(9),
                    HandCard::ObjectiveCard(24),
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
                //can't fulfill all objectives with same name
                assert_eq!(c.choices.len(), 8);
                assert!(validate_card_selection(&c.choices, game).is_err());
            }),
        ],
    )
}

#[test]
fn test_colony() {
    // Game ends after this step
    JSON.test(
        "colony",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn))
                .without_json_comparison(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(17),
                    HandCard::ObjectiveCard(32),
                ])),
            ),
        ],
    )
}

#[test]
fn test_standing_army() {
    JSON.test(
        "standing_army",
        vec![
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(14),
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
                //can't fulfill all objectives with same name
                assert_eq!(c.choices.len(), 2);
                assert!(validate_card_selection(&c.choices, game).is_err());
            }),
        ],
    )
}
