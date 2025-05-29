use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::playing_actions::{PlayingAction, Recruit};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "greece");

#[test]
fn sparta_draft() {
    JSON.test("sparta_draft", vec![TestAction::undoable(
        0,
        Action::Playing(PlayingAction::Recruit(Recruit::new(
            &Units::new(0, 1, 0, 0, 0, 0),
            Position::from_offset("A1"),
            ResourcePile::culture_tokens(1),
        ))),
    )]);
}

#[test]
fn sparta_battle() {
    JSON.test("sparta_battle", vec![TestAction::not_undoable(
        0,
        move_action(vec![0], Position::from_offset("C1")),
    )]);
}
