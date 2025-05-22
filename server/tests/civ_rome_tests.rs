use crate::common::{JsonTest, TestAction, advance_action, custom_action, payment_response};
use server::action::Action;
use server::advance::Advance;
use server::city_pieces::Building;
use server::construct::Construct;
use server::content::custom_actions::CustomActionType;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "rome");

#[test]
fn aqueduct_discount() {
    JSON.test("aqueduct_discount", vec![TestAction::undoable(
        0,
        advance_action(Advance::Sanitation, ResourcePile::empty()),
    )]);
}

#[test]
fn aqueduct_free_action() {
    JSON.test("aqueduct_free_action", vec![
        TestAction::undoable(0, custom_action(CustomActionType::Aqueduct)).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::food(2))),
    ]);
}

#[test]
fn roman_roads() {
    JSON.test("roman_roads", vec![TestAction::undoable(
        0,
        Action::Movement(MovementAction::Move(MoveUnits::new(
            vec![0, 1, 2, 3, 4, 5],
            Position::from_offset("A1"),
            None,
            ResourcePile::food(1) + ResourcePile::ore(1),
        ))),
    )]);
}

#[test]
fn captivi() {
    JSON.test("captivi", vec![TestAction::undoable(
        0,
        Action::Playing(PlayingAction::Construct(Construct::new(
            Position::from_offset("A1"),
            Building::Market,
            ResourcePile::new(1, 1, 0, 0, 0, 1, 0),
        ))),
    )]);
}
