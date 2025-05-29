use crate::common::{JsonTest, TestAction, move_action, payment_response};
use server::action::Action;
use server::city_pieces::Building;
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::{EventResponse, SelectedStructure, Structure};
use server::cultural_influence::{InfluenceCultureAttempt, affordable_start_city};
use server::playing_actions::{PlayingAction, PlayingActionType, Recruit};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "greece");

#[test]
fn sparta_draft() {
    JSON.test(
        "sparta_draft",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::Recruit(Recruit::new(
                &Units::new(0, 1, 0, 0, 0, 0),
                Position::from_offset("A1"),
                ResourcePile::culture_tokens(1),
            ))),
        )],
    );
}

#[test]
fn sparta_battle() {
    JSON.test(
        "sparta_battle",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn hellenistic_culture_staring_point() {
    let game = &JSON.load_game("hellenistic_culture");

    assert_eq!(
        affordable_start_city(
            game,
            0,
            game.get_any_city(Position::from_offset("D1")),
            None,
        )
        .unwrap(),
        (Position::from_offset("C2"), 0)
    );
    assert_eq!(
        affordable_start_city(
            game,
            0,
            game.get_any_city(Position::from_offset("C2")),
            None,
        )
        .unwrap(),
        (Position::from_offset("C2"), 0)
    );
}

#[test]
fn hellenistic_culture_cost() {
    JSON.test(
        "hellenistic_culture",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::InfluenceCultureAttempt(
                    InfluenceCultureAttempt::new(
                        SelectedStructure::new(
                            Position::from_offset("C2"),
                            Structure::Building(Building::Port),
                        ),
                        PlayingActionType::Custom(
                            CustomActionType::HellenisticInfluenceCultureAttempt,
                        ),
                    ),
                )),
            )
            .skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::mood_tokens(2))),
        ],
    )
}

#[test]
fn city_states() {
    JSON.test(
        "city_states",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Recruit(Recruit::new(
                    &Units::new(0, 1, 0, 0, 0, 0),
                    Position::from_offset("A1"),
                    ResourcePile::culture_tokens(1),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B3",
                )])),
            ),
        ],
    )
}
