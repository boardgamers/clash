use crate::common::{advance_action, custom_action, payment_response, JsonTest, TestAction};
use server::advance::Advance;
use server::content::custom_actions::CustomActionType;
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
