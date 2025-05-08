use crate::common::{JsonTest, TestAction, move_action};
use server::action::{Action, execute_without_undo};
use server::advance::Advance;
use server::card::HandCard;
use server::content::custom_actions::{CustomActionType, CustomEventAction};
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::playing_actions::PlayingAction::WonderCard;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::wonder::Wonder;

mod common;

const JSON: JsonTest = JsonTest::new("wonders");

#[test]
fn test_colosseum() {
    JSON.test(
        "colosseum",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Colosseum)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    3, 4, 5, 0, 0, 0, 5,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Custom(CustomEventAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                // mood payment is only possible because of colosseum
                Action::Response(EventResponse::Payment(vec![ResourcePile::mood_tokens(1)])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0]))),
        ],
    );
}

#[test]
fn test_pyramids() {
    JSON.test(
        "pyramids",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Pyramids)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    2, 3, 6, 0, 1, 0, 5,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(32),
                ])),
            ),
        ],
    );
}

#[test]
fn test_library() {
    JSON.test(
        "library",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Custom(CustomEventAction::new(
                    CustomActionType::GreatLibrary,
                    None,
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(Advance::Engineering)),
            )
            .without_json_comparison(),
            // can use effect to build a wonder - but don't draw a wonder card (one time ability)
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Pyramids))),
        ],
    );
}

#[test]
fn test_lighthouse() {
    JSON.test(
        "lighthouse",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatLighthouse)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    3, 5, 4, 0, 0, 0, 5,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Custom(CustomEventAction::new(
                    CustomActionType::GreatLighthouse,
                    None,
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C3",
                )])),
            ),
        ],
    );
}

#[test]
fn test_great_gardens() {
    JSON.test(
        "great_gardens",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatGardens)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    5, 5, 2, 0, 0, 0, 5,
                )])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn))
                .without_json_comparison(),
            TestAction::undoable(1, move_action(vec![0, 1], Position::from_offset("B1")))
                .with_post_assert(|mut game| {
                    let result = execute_without_undo(
                        &mut game,
                        move_action(vec![0, 1], Position::from_offset("A1")),
                        1,
                    );
                    assert_eq!(
                        result.err(),
                        Some("fertile movement attack great gardens restriction".to_string())
                    );
                }),
        ],
    );
}

#[test]
fn test_great_mausoleum() {
    JSON.test(
        "great_mausoleum",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(123)))
                .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true)))
                .without_json_comparison(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Advance {
                    advance: Advance::Storage,
                    payment: ResourcePile::ideas(2),
                }),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

#[test]
fn test_great_statue() {
    JSON.test(
        "great_statue",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatStatue)))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                    3, 4, 5, 0, 0, 0, 5,
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Custom(CustomEventAction::new(
                    CustomActionType::GreatStatue,
                    None,
                ))),
            ),
        ],
    );
}

#[test]
fn test_great_wall() {
    JSON.test(
        "great_wall",
        vec![
            TestAction::not_undoable(1, move_action(vec![0, 1], Position::from_offset("A1")))
                .without_json_comparison(),
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Playing(PlayingAction::Advance {
                    advance: Advance::Storage,
                    payment: ResourcePile::ideas(2),
                }),
            )
            .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(5))),
        ],
    );
}
