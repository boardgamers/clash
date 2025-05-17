use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::content::persistent_events::EventResponse;
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "rome");

#[test]
fn aqueduct_discount() {
    JSON.test(
        "aqueduct_discount",
        vec![
            TestAction::not_undoable(0, move_action(vec![10], Position::from_offset("B1"))),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B2",
                )])),
            ),
        ],
    );
}
