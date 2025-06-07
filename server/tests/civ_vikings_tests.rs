use crate::common::{
    move_action, JsonTest, TestAction,
};
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn convert_to_ship() {
    JSON.test("convert_to_ship", vec![TestAction::undoable(
        0,
        move_action(vec![3], Position::from_offset("D2")),
    )]);
}
