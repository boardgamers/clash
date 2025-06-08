use crate::common::{JsonTest, TestAction, custom_action, move_action, payment_response};
use server::action::Action;
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::EventResponse;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::PlayingAction::EndTurn;
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn ship_construction() {
    JSON.test(
        "ship_construction",
        vec![
            TestAction::undoable(0, move_action(vec![3], Position::from_offset("D2"))).skip_json(),
            // embark in same current move
            TestAction::undoable(
                0,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    vec![4, 5],
                    Position::from_offset("D2"),
                    Some(3),
                    ResourcePile::empty(),
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, move_action(vec![3], Position::from_offset("C2"))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![3]))),
        ],
    );
}

#[test]
fn longships() {
    JSON.test(
        "longships",
        vec![
            TestAction::not_undoable(0, move_action(vec![3, 4, 5], Position::from_offset("C3")))
                .skip_json(),
            TestAction::not_undoable(0, move_action(vec![4, 5], Position::from_offset("C4"))),
        ],
    )
}

#[test]
fn raids() {
    JSON.test(
        "raids",
        vec![
            TestAction::not_undoable(0, Action::Playing(EndTurn)).skip_json(),
            TestAction::not_undoable(1, Action::Playing(EndTurn)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::food(1))),
        ],
    );
}

#[test]
fn danegeld() {
    JSON.test(
        "danegeld",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Danegeld)).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::food(4))),
            ),
        ],
    );
}

#[test]
fn explorer() {
    JSON.test(
        "explorer",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::LegendaryExplorer)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}
