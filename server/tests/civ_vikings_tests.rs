use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::content::persistent_events::EventResponse;
use server::movement::{MoveUnits, MovementAction};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn ship_construction() {
    let destination = Position::from_offset("D2");
    JSON.test(
        "ship_construction",
        vec![
            TestAction::undoable(0, move_action(vec![3], Position::from_offset("D2"))).skip_json(),
            // embark in same current move
            TestAction::undoable(
                0,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    vec![4, 5],
                    destination,
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
