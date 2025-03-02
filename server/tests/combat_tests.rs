// combat

use crate::common::{move_action, test_action, test_actions, TestAction};
use server::action::Action;
use server::content::custom_phase_actions::CurrentEventResponse;
use server::playing_actions::PlayingAction::Recruit;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

#[test]
fn test_remove_casualties_attacker() {
    test_actions(
        "remove_casualties_attacker",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnits(vec![0, 1])),
            ),
        ],
    );
}

#[test]
fn test_remove_casualties_defender() {
    test_actions(
        "remove_casualties_defender",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_metallurgy() {
    test_action(
        "direct_capture_city_metallurgy",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_direct_capture_city_fortress() {
    test_action(
        "direct_capture_city_fortress",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_direct_capture_city_only_fortress() {
    test_action(
        "direct_capture_city_only_fortress",
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
        false,
        false,
    );
}

#[test]
fn test_combat_all_modifiers() {
    test_actions(
        "combat_all_modifiers",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![ResourcePile::ore(1)])),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![
                    ResourcePile::empty(),
                    ResourcePile::ore(2),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![ResourcePile::ore(1)])),
            ),
        ],
    );
}

#[test]
fn test_combat_fanaticism() {
    test_actions(
        "combat_fanaticism",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_retreat() {
    test_actions(
        "retreat",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Bool(true)),
            ),
        ],
    );
}

#[test]
fn test_do_not_retreat() {
    test_actions(
        "retreat_no",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Bool(false)),
            ),
        ],
    );
}

#[test]
fn test_ship_combat() {
    test_actions(
        "ship_combat",
        vec![
            TestAction::not_undoable(0, move_action(vec![7, 8], Position::from_offset("D2"))),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnits(vec![1])),
            ),
        ],
    );
}

#[test]
fn test_ship_combat_war_ships() {
    test_action(
        "ship_combat_war_ships",
        move_action(vec![7, 8], Position::from_offset("D2")),
        0,
        false,
        false,
    );
}

#[test]
fn test_recruit_combat() {
    test_actions(
        "recruit_combat",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::playing_actions::Recruit {
                    units: Units::new(0, 0, 3, 0, 0, 0),
                    city_position: Position::from_offset("C2"),
                    payment: ResourcePile::wood(5) + ResourcePile::gold(1),
                    leader_name: None,
                    replaced_units: vec![],
                })),
            ),
            TestAction::undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::ResourceReward(
                    ResourcePile::mood_tokens(1),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::ResourceReward(ResourcePile::gold(
                    1,
                ))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::ResourceReward(
                    ResourcePile::culture_tokens(1),
                )),
            ),
        ],
    );
}
