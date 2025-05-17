use crate::common::{JsonTest, TestAction, advance_action};
use server::advance::Advance;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "rome");

#[test]
fn aqueduct_discount() {
    JSON.test(
        "aqueduct_discount",
        vec![TestAction::undoable(
            0,
            advance_action(Advance::Sanitation, ResourcePile::empty()),
        )],
    );
}
