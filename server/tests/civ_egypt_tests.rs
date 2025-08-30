use crate::common::{JsonTest, TestAction};
use server::action::Action;
use server::collect::{Collect, PositionCollection};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "egypt");

#[test]
fn flood() {
    JSON.test(
        "flood",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Collect(Collect::new(
                    Position::from_offset("B3"),
                    vec![PositionCollection::new(
                        Position::from_offset("B4"),
                        ResourcePile::wood(1),
                    )],
                    PlayingActionType::Collect,
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::FoundCity { settler: 1 })),
        ],
    )
}
