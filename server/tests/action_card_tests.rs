use crate::common::{move_action, TestAction};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::custom_phase_actions::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::new("action_cards");

#[test]
fn test_advance() {
    JSON.test(
        "advance",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(2))),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectAdvance("Storage".to_string())),
            ),
        ],
    );
}

#[test]
fn test_inspiration() {
    JSON.test(
        "inspiration",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::ActionCard(3)),
        )],
    );
}

#[test]
fn test_hero_general() {
    JSON.test(
        "hero_general",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .without_json_comparison(),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectHandCards(vec![])))
                .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(5)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C1",
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::mood_tokens(1)])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C2",
                )])),
            ),
        ],
    );
}

#[test]
fn test_spy() {
    JSON.test(
        "spy",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::ActionCard(7)))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ActionCard(1),
                    HandCard::ActionCard(2),
                ])),
            ),
        ],
    );
}
