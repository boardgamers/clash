use crate::common::{move_action, test_actions, TestAction};
use server::action::Action;
use server::content::custom_phase_actions::CurrentEventResponse;
use server::position::Position;

mod common;

#[test]
fn test_civ_maya_leader_pakal_and_place_settler() {
    test_actions(
        "civ_maya_leader_pakal",
        vec![
            TestAction::not_undoable(0, move_action(vec![10], Position::from_offset("B1"))),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("B2"),
                ])),
            ),
        ],
    );
}
