use crate::common::{TestAction, move_action, payment_response};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::city_pieces::Building::Fortress;
use server::collect::PositionCollection;
use server::content::persistent_events::{EventResponse, SelectedStructure, Structure};
use server::movement::possible_move_units_destinations;
use server::playing_actions::PlayingAction::Construct;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::{advance, construct, cultural_influence, playing_actions};

mod common;

const JSON: JsonTest = JsonTest::new("action_cards");

#[test]
fn test_advance() {
    JSON.test("advance", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(2))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::Storage)),
        ),
    ]);
}

#[test]
fn test_inspiration() {
    JSON.test("inspiration", vec![TestAction::undoable(
        0,
        Action::Playing(PlayingAction::ActionCard(3)),
    )]);
}

#[test]
fn test_hero_general() {
    JSON.test("hero_general", vec![
        TestAction::undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )
        .skip_json(),
        TestAction::not_undoable(0, Action::Response(EventResponse::SelectHandCards(vec![]))),
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(5))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "C1",
            )])),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::mood_tokens(1)),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "C2",
            )])),
        ),
    ]);
}

#[test]
fn test_spy() {
    JSON.test("spy", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(7))).skip_json(),
        TestAction::not_undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectHandCards(vec![
                HandCard::ActionCard(1),
                HandCard::ActionCard(2),
            ])),
        ),
    ]);
}

#[test]
fn test_ideas() {
    JSON.test("ideas", vec![TestAction::undoable(
        0,
        Action::Playing(PlayingAction::ActionCard(9)),
    )]);
}

#[test]
fn test_great_ideas() {
    JSON.test("great_ideas", vec![
        TestAction::undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )
        .skip_json(),
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(11))),
    ]);
}

#[test]
fn test_mercenaries() {
    JSON.test("mercenaries", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(13))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![
                Position::from_offset("A3"),
                Position::from_offset("B3"),
            ])),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::ore(2)),
        )
        .skip_json(),
        TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "A3",
            )])),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "B2",
            )])),
        ),
    ]);
}

#[test]
fn test_cultural_takeover() {
    JSON.test("cultural_takeover", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(15))).skip_json(),
        TestAction::undoable(
            0,
            Action::Playing(PlayingAction::InfluenceCultureAttempt(
                cultural_influence::InfluenceCultureAttempt::new(
                    SelectedStructure::new(Position::from_offset("B3"), Structure::CityCenter),
                    PlayingActionType::InfluenceCultureAttempt,
                ),
            )),
        )
        .skip_json(),
        TestAction::not_undoable(0, payment_response(ResourcePile::culture_tokens(2))),
    ]);
}

#[test]
fn test_city_development() {
    JSON.test("city_development", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(17))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::undoable(
            0,
            Action::Playing(Construct(construct::Construct::new(
                Position::from_offset("C2"),
                Fortress,
                ResourcePile::empty(),
            ))),
        ),
    ]);
}

#[test]
fn test_production_focus() {
    JSON.test("production_focus", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(19))).skip_json(),
        TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Collect(playing_actions::Collect::new(
                Position::from_offset("C2"),
                vec![
                    PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1))
                        .times(2),
                    PositionCollection::new(
                        Position::from_offset("C3"),
                        ResourcePile::mood_tokens(1),
                    )
                    .times(2),
                    PositionCollection::new(Position::from_offset("C3"), ResourcePile::gold(1)),
                ],
                PlayingActionType::Collect,
            ))),
        ),
    ]);
}

#[test]
fn test_explorer() {
    JSON.test("explorer", vec![
        TestAction::undoable(1, Action::Playing(PlayingAction::ActionCard(21))).skip_json(),
        TestAction::undoable(1, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::not_undoable(
            1,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "B6",
            )])),
        )
        .skip_json(),
        TestAction::undoable(1, Action::Response(EventResponse::ExploreResolution(0))).skip_json(),
        TestAction::undoable(
            1,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "D8",
            )])),
        ),
    ]);
}

#[test]
fn test_negotiations() {
    JSON.test("negotiations", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(23))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn))
            .with_pre_assert(|game| {
                assert!(
                    !possible_move_units_destinations(
                        game.player(0),
                        &game,
                        &[0],
                        Position::from_offset("C2"),
                        None,
                    )
                    .iter()
                    .any(|r| r
                        .iter()
                        .any(|r| r.destination == Position::from_offset("B1")))
                );
            })
            .skip_json(),
        TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn))
            .with_pre_assert(|game| {
                assert!(
                    !possible_move_units_destinations(
                        game.player(1),
                        &game,
                        &[0],
                        Position::from_offset("B1"),
                        None,
                    )
                    .iter()
                    .any(|r| r
                        .iter()
                        .any(|r| r.destination == Position::from_offset("C2")))
                );
            })
            .skip_json(),
        TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("B1"))),
    ]);
}

#[test]
fn test_assassination() {
    JSON.test("assassination", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(27))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)).skip_json(),
        TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn)).with_pre_assert(
            |game| {
                assert_eq!(game.actions_left, 2);
            },
        ),
    ]);
}

#[test]
fn test_overproduction() {
    JSON.test("overproduction", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(29))).skip_json(),
        TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Collect(playing_actions::Collect::new(
                Position::from_offset("C2"),
                vec![
                    PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1)),
                    PositionCollection::new(Position::from_offset("B2"), ResourcePile::wood(1)),
                    PositionCollection::new(
                        Position::from_offset("C3"),
                        ResourcePile::mood_tokens(1),
                    ),
                ],
                PlayingActionType::Collect,
            ))),
        ),
    ]);
}

#[test]
fn test_new_plans() {
    JSON.test("new_plans", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(31))).skip_json(),
        TestAction::not_undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectHandCards(vec![
                HandCard::ObjectiveCard(3),
                HandCard::ObjectiveCard(2),
            ])),
        ),
    ]);
}

#[test]
fn test_synergies() {
    JSON.test("synergies", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(34))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::Cartography)),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::ideas(2)),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::WarShips)),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::ideas(2)),
        ),
    ]);
}

#[test]
fn test_teach_us() {
    JSON.test("teach_us", vec![
        TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )
        .skip_json(),
        TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                35,
            )])),
        ),
    ]);
}

#[test]
fn test_militia() {
    JSON.test("militia", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(37))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
    ]);
}

#[test]
fn test_tech_trade() {
    JSON.test("tech_trade", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(39))).skip_json(),
        TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
    ]);
}

#[test]
fn test_new_ideas() {
    JSON.test("new_ideas", vec![
        TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(41))).skip_json(),
        TestAction::undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(advance::Advance::Storage)),
        )
        .skip_json(),
        TestAction::undoable(
            0,
            payment_response(ResourcePile::food(2)),
        ),
    ]);
}
