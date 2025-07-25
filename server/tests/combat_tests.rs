// combat

use crate::common::{TestAction, move_action, payment_response};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction::Recruit;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::new("combat");

#[test]
fn test_remove_casualties_attacker() {
    JSON.test(
        "remove_casualties_attacker",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            ),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0, 1]))),
        ],
    );
}

#[test]
fn test_remove_casualties_defender() {
    JSON.test(
        "remove_casualties_defender",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_metallurgy() {
    JSON.test(
        "direct_capture_city_metallurgy",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_fortress() {
    JSON.test(
        "direct_capture_city_fortress",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_capture_and_raze_city() {
    JSON.test(
        "capture_and_raze_city",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_only_fortress() {
    JSON.test(
        "direct_capture_city_only_fortress",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_combat_all_modifiers() {
    JSON.test(
        "combat_all_modifiers",
        vec![
            TestAction::undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::ore(1))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![
                    ResourcePile::empty(),
                    ResourcePile::ore(2),
                ])),
            )
            .skip_json(),
            TestAction::not_undoable(1, payment_response(ResourcePile::ore(1))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    2,
                )])),
            ),
        ],
    );
}

#[test]
fn test_combat_fanaticism() {
    JSON.test(
        "combat_fanaticism",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_retreat() {
    JSON.test(
        "retreat",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

#[test]
fn test_do_not_retreat() {
    JSON.test(
        "retreat_no",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))),
        ],
    );
}

#[test]
fn test_ship_combat() {
    JSON.test(
        "ship_combat",
        vec![
            TestAction::not_undoable(0, move_action(vec![7, 8], Position::from_offset("D2"))),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![1]))),
        ],
    );
}

#[test]
fn test_ship_combat_war_ships() {
    JSON.test(
        "ship_combat_war_ships",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![7, 8], Position::from_offset("D2")),
        )],
    );
}

#[test]
fn test_recruit_combat() {
    JSON.test(
        "recruit_combat",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::recruit::Recruit::new(
                    &Units::new(0, 0, 4, 0, 0, None),
                    Position::from_offset("C2"),
                    ResourcePile::wood(5) + ResourcePile::gold(3),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::mood_tokens(1))),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::gold(1))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            )
            .skip_json(),
            TestAction::not_undoable(0, move_action(vec![12, 13], Position::from_offset("C4")))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(38),
                ])),
            ),
        ],
    );
}
