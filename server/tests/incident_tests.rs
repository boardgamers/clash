use crate::common::{move_action, test_actions, TestAction};
use server::action::Action;
use server::city_pieces::Building::Fortress;
use server::content::custom_phase_actions::CustomPhaseEventAction;
use server::playing_actions;
use server::playing_actions::PlayingAction::{Advance, Construct};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::StatusPhaseAction;
use server::unit::UnitType;

mod common;

#[test]
fn test_barbarians_spawn() {
    test_actions(
        "barbarians_spawn",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("B3"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectUnitType(
                    UnitType::Elephant,
                )),
            ),
        ],
    );
}

#[test]
fn test_barbarians_move() {
    test_actions(
        "barbarians_move",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("B3"),
                )),
            ),
        ],
    );
}

#[test]
fn test_pirates_spawn() {
    test_actions(
        "pirates_spawn",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectUnits(vec![7])),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("A2"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("D2"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::Payment(vec![ResourcePile::ore(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_barbarians_attack() {
    test_actions(
        "barbarians_attack",
        vec![TestAction::not_undoable(
            0,
            Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
        )],
    );
}

#[test]
fn test_barbarians_recapture_city() {
    test_actions(
        "barbarians_recapture_city",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![5, 6, 7, 8], Position::from_offset("C2")),
        )],
    );
}

#[test]
fn test_pestilence() {
    let cons = Action::Playing(Construct(playing_actions::Construct {
        city_position: Position::from_offset("C2"),
        city_piece: Fortress,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        port_position: None,
    }));
    test_actions(
        "pestilence",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CustomPhaseEventAction::SelectPosition(
                    Position::from_offset("A1"),
                )),
            ),
            TestAction::illegal(0, cons.clone()).without_json_comparison(),
            TestAction::undoable(
                //no compare
                0,
                Action::Playing(Advance {
                    advance: String::from("Sanitation"),
                    payment: ResourcePile::gold(2),
                }),
            )
            .without_json_comparison(),
            TestAction::undoable(0, cons).without_json_comparison(),
        ],
    );
}
