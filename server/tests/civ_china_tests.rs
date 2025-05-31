use crate::common::{JsonTest, TestAction};
use server::action::Action;
use server::collect::PositionCollection;
use server::playing_actions::{Collect, PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "china");

#[test]
fn rice() {
    JSON.test(
        "rice",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Collect(Collect::new(
                Position::from_offset("B3"),
                vec![PositionCollection::new(
                    Position::from_offset("B4"),
                    ResourcePile::food(1),
                )],
                PlayingActionType::Collect,
            ))),
        )],
    );
}
