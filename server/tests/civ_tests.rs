use crate::common::{move_action, JsonTest, TestAction};
use server::action::Action;
use server::content::custom_phase_actions::EventResponse;
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::new("civ");

#[test]
fn test_civ_maya_leader_pakal_and_place_settler() {
    JSON.test(
        "civ_maya_leader_pakal",
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
