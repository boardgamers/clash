use crate::common::{move_action, JsonTest, TestAction};
use server::action::Action;
use server::city_pieces::Building::Fortress;
use server::construct;
use server::content::custom_phase_actions::EventResponse;
use server::playing_actions::PlayingAction::{Advance, Construct};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const GREAT_PERSONS: JsonTest = JsonTest::child("incidents", "great_persons");

#[test]
fn test_great_explorer() {
    GREAT_PERSONS.test(
        "great_explorer",
        vec![
            TestAction::not_undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            ),
            TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(118))),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B6",
                )])),
            ),
            TestAction::undoable(1, Action::Response(EventResponse::ExploreResolution(0))),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B6",
                )])),
            ),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::food(2)])),
            ),
        ],
    );
}

#[test]
fn test_great_artist() {
    GREAT_PERSONS.test(
        "great_artist",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    2,
                )])),
            ),
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
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(120)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C1",
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::gold(2)])),
            ),
        ],
    );
}

#[test]
fn test_great_warlord() {
    GREAT_PERSONS.test(
        "great_warlord",
        vec![
            TestAction::not_undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(124)))
                .without_json_comparison(),
            TestAction::not_undoable(1, move_action(vec![0], Position::from_offset("C8"))),
        ],
    );
}

#[test]
fn test_great_merchant() {
    GREAT_PERSONS.test(
        "great_merchant",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(125)))
                .without_json_comparison(),
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
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(126)))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance("Engineering".to_string())),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true)))
                .without_json_comparison(),
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
                assert!(PlayingActionType::Advance.is_available(game, 0).is_err())
            }),
        ],
    );
}

#[test]
fn test_great_architect() {
    GREAT_PERSONS.test(
        "great_architect",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(155)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    2, 3, 3, 0, 0, 0, 1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_great_athlete() {
    GREAT_PERSONS.test(
        "great_athlete",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::food(2),
                }),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(156)))
                .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            ),
        ],
    );
}
