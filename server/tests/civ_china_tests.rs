use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, move_action, payment_response,
};
use server::action::Action;
use server::advance::Advance;
use server::collect::{Collect, PositionCollection};
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction::WonderCard;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::recruit::Recruit;
use server::resource_pile::ResourcePile;
use server::unit::Units;
use server::wonder::Wonder;

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

#[test]
fn fast_war() {
    JSON.test(
        "fast_war",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("D8")),
        )],
    );
}

#[test]
fn great_wall() {
    JSON.test(
        "great_wall",
        vec![
            TestAction::undoable(
                0,
                advance_action(Advance::Engineering, ResourcePile::food(2)),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatWall))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(3, 2, 7, 0, 0, 0, 3)))
                .skip_json(),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::undoable(1, Action::Response(EventResponse::Bool(true))),
        ],
    );
}
