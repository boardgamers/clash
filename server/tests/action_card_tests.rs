use crate::common::{move_action, TestAction};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::city_pieces::Building::Fortress;
use server::collect::PositionCollection;
use server::content::custom_phase_actions::{EventResponse, SelectedStructure, Structure};
use server::playing_actions::PlayingAction;
use server::playing_actions::PlayingAction::Construct;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::{construct, playing_actions};

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
                Action::Response(EventResponse::SelectAdvance("Storage".to_string())),
            ),
        ],
    );
}

#[test]
fn test_inspiration() {
    JSON.test(
        "inspiration",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::ActionCard(3)),
        )],
    );
}

#[test]
fn test_hero_general() {
    JSON.test(
        "hero_general",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .without_json_comparison(),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectHandCards(vec![]))),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(5)))
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
                Action::Response(EventResponse::Payment(vec![ResourcePile::mood_tokens(1)])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C2",
                )])),
            ),
        ],
    );
}

#[test]
fn test_spy() {
    JSON.test(
        "spy",
        vec![
            TestAction::not_undoable(0, Action::Playing(PlayingAction::ActionCard(7)))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ActionCard(1),
                    HandCard::ActionCard(2),
                ])),
            ),
        ],
    );
}

#[test]
fn test_ideas() {
    JSON.test(
        "ideas",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::ActionCard(9)),
        )],
    );
}

#[test]
fn test_great_ideas() {
    JSON.test(
        "great_ideas",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .without_json_comparison(),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectHandCards(vec![])))
                .without_json_comparison(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(11))),
        ],
    );
}

#[test]
fn test_mercenaries() {
    JSON.test(
        "mercenaries",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(13)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![
                    Position::from_offset("A3"),
                    Position::from_offset("B3"),
                ])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::ore(2)])),
            )
            .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A3",
                )])),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B2",
                )])),
            ),
        ],
    );
}

#[test]
fn test_cultural_takeover() {
    JSON.test(
        "cultural_takeover",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(15)))
                .without_json_comparison(),
            TestAction::not_undoable(
                0,
                Action::Playing(PlayingAction::InfluenceCultureAttempt(
                    SelectedStructure::new(Position::from_offset("B3"), Structure::CityCenter),
                )),
            ),
        ],
    );
}

#[test]
fn test_city_development() {
    JSON.test(
        "city_development",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(17)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(Construct(construct::Construct::new(
                    Position::from_offset("C2"),
                    Fortress,
                    ResourcePile::empty(),
                ))),
            ),
        ],
    );
}

#[test]
fn test_production_focus() {
    JSON.test(
        "production_focus",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(19)))
                .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Collect(playing_actions::Collect {
                    city_position: Position::from_offset("C2"),
                    collections: vec![
                        PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1))
                            .times(2),
                        PositionCollection::new(
                            Position::from_offset("C3"),
                            ResourcePile::mood_tokens(1),
                        )
                        .times(2),
                        PositionCollection::new(Position::from_offset("C3"), ResourcePile::gold(1)),
                    ],
                })),
            ),
        ],
    );
}
