use crate::common::{JsonTest, TestAction, advance_action, move_action, payment_response};
use advance::Advance;
use server::action::Action;
use server::card::HandCard;
use server::city_pieces::Building::Fortress;
use server::content::persistent_events::EventResponse;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::PlayingAction::Construct;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::{advance, construct};

mod common;

const GREAT_PERSONS: JsonTest = JsonTest::child("incidents", "great_persons");

#[test]
fn test_great_explorer() {
    GREAT_PERSONS.test(
        "great_explorer",
        vec![
            TestAction::not_undoable(1, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(1, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(118))).skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B6",
                )])),
            )
            .skip_json(),
            TestAction::undoable(1, Action::Response(EventResponse::ExploreResolution(0)))
                .skip_json(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A7",
                )])),
            )
            .skip_json(),
            TestAction::undoable(1, payment_response(ResourcePile::food(2))),
        ],
    );
}

#[test]
fn test_great_artist() {
    GREAT_PERSONS.test(
        "great_artist",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2))),
            TestAction::not_undoable(1, payment_response(ResourcePile::culture_tokens(2))),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)),
            TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(119))),
        ],
    );
}

#[test]
fn test_great_prophet() {
    GREAT_PERSONS.test(
        "great_prophet",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(120))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C1",
                )])),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::gold(2))),
        ],
    );
}

#[test]
fn test_great_warlord() {
    GREAT_PERSONS.test(
        "great_warlord",
        vec![
            TestAction::not_undoable(1, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(1, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(124))).skip_json(),
            TestAction::not_undoable(1, move_action(vec![0], Position::from_offset("C8"))),
        ],
    );
}

#[test]
fn test_great_merchant() {
    GREAT_PERSONS.test(
        "great_merchant",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(125))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::gold(1))),
            ),
        ],
    );
}

#[test]
fn test_great_engineer() {
    GREAT_PERSONS.test(
        "great_engineer",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(126))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(Advance::Engineering)),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))).skip_json(),
            TestAction::undoable(
                0,
                Action::Playing(Construct(construct::Construct::new(
                    Position::from_offset("C2"),
                    Fortress,
                    ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
                ))),
            )
            .with_pre_assert(|game| {
                // must do construct
                assert!(PlayingActionType::Advance.is_available(&game, 0).is_err())
            }),
        ],
    );
}

#[test]
fn test_great_architect() {
    GREAT_PERSONS.test(
        "great_architect",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(155))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A1",
                )])),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(2, 3, 6, 0, 1, 0, 2))),
        ],
    );
}

#[test]
fn test_great_athlete() {
    GREAT_PERSONS.test(
        "great_athlete",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(156))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn test_great_diplomat() {
    let units = vec![0];
    let destination = Position::from_offset("B1");
    GREAT_PERSONS.test(
        "great_diplomat",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    units,
                    destination,
                    None,
                    ResourcePile::culture_tokens(2),
                ))),
            ),
        ],
    );
}

#[test]
fn test_great_seer() {
    GREAT_PERSONS.test(
        "great_seer",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::food(2)))
                .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::ActionCard(158)))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(21),
                ])),
            )
            .skip_json(),
            // the player already knows the card - but we treat all designated cards as unknown
            TestAction::not_undoable(0, advance_action(Advance::Writing, ResourcePile::food(2))),
        ],
    );
}
