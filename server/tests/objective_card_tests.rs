use crate::common::{JsonTest, TestAction};
use server::action::Action;
use server::card::{validate_card_selection, HandCard};
use server::content::persistent_events::{EventResponse, PersistentEventRequest};
use server::playing_actions;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::new("objective_cards");

#[test]
fn test_draft() {
    let r = Action::Playing(PlayingAction::Recruit(playing_actions::Recruit::new(
        &Units::new(0, 1, 0, 0, 0, 0),
        Position::from_offset("A1"),
        ResourcePile::mood_tokens(1),
    )));
    JSON.test("draft", vec![
        TestAction::undoable(0, r.clone()).without_json_comparison(),
        TestAction::undoable(0, r).without_json_comparison(),
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
    ])
}
