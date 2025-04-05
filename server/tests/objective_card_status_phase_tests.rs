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
    )
}
