use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, move_action, payment_response,
};
use server::action::Action;
use server::advance::Advance;
use server::city_pieces::Building;
use server::construct::Construct;
use server::content::custom_actions::CustomActionType;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::{IncreaseHappiness, PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn ship_construction() {
    JSON.test(
        "ship_construction",
        vec![TestAction::undoable(
            0,
            advance_action(Advance::Sanitation, ResourcePile::empty()),
        )],
    );
}
