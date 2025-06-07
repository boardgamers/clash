use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::movement::{MoveUnits, MovementAction};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn convert_to_ship() {
    let units = vec![4, 5];
    let destination = Position::from_offset("D2");
    JSON.test("convert_to_ship", vec![
        TestAction::undoable(0, move_action(vec![3], Position::from_offset("D2"))),
        // embark in same current move
        TestAction::undoable(
            0,
            Action::Movement(MovementAction::Move(MoveUnits::new(
                units,
                destination,
                Some(3),
                ResourcePile::empty(),
            ))),
        ),
    ]);
}
