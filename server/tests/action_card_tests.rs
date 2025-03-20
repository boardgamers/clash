use crate::common::TestAction;
use common::JsonTest;
use server::action::Action;
use server::content::custom_phase_actions::CurrentEventResponse;
use server::playing_actions::PlayingAction;

mod common;

const JSON: JsonTest = JsonTest::new("action_cards");

#[test]
fn test_advance() {
    JSON.test(
        "advance",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(2))),
            TestAction::undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
        ],
    );
}
