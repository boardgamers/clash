use crate::common::{
    JsonTest, TestAction,
};
use server::action::Action;
use server::playing_actions::{PlayingAction, Recruit};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "greece");

#[test]
fn aqueduct_discount() {
    JSON.test("sparta_draft", vec![TestAction::undoable(
        0,
        Action::Playing(PlayingAction::Recruit(Recruit::new(
            &Units::new(0, 1, 0, 0, 0, 0),
            Position::from_offset("A1"),
            ResourcePile::culture_tokens(1),
        ))),
    )]);
}
