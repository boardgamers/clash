use crate::common::{JsonTest, TestAction, custom_action, move_action, payment_response};
use server::action::Action;
use server::collect::PositionCollection;
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::EventResponse;
use server::playing_actions::{Collect, PlayingAction, PlayingActionType, Recruit};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

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

#[test]
fn expansion() {
    JSON.test(
        "expansion",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Recruit(Recruit::new(
                &Units::new(1, 0, 0, 0, 0, None),
                Position::from_offset("A1"),
                ResourcePile::gold(2),
            ))),
        )],
    );
}

#[test]
fn fireworks() {
    JSON.test(
        "fireworks",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::gold(2))),
        ],
    );
}

#[test]
fn imperial_army() {
    JSON.test(
        "imperial_army",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::ImperialArmy)).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0, 4]))),
        ],
    );
}
