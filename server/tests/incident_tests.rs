use crate::common::{move_action, test_actions, TestAction};
use server::action::Action;
use server::city_pieces::Building::Fortress;
use server::content::custom_phase_actions::{CurrentEventResponse, Structure};
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("B3"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnitType(UnitType::Elephant)),
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnits(vec![7])),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("A2"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("D2"),
                )),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![ResourcePile::ore(1)])),
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("A1"),
                )),
            ),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![
                    ResourcePile::mood_tokens(1),
                ])),
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

#[test]
fn test_famine() {
    test_actions(
        "famine",
        vec![TestAction::not_undoable(
            0,
            Action::Playing(Advance {
                advance: String::from("Storage"),
                payment: ResourcePile::gold(2),
            }),
        )],
    );
}

#[test]
fn test_epidemics() {
    test_actions(
        "epidemics",
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnits(vec![7])),
            ),
        ],
    );
}

#[test]
fn test_good_year_with_player_select() {
    test_actions(
        "good_year",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectUnitType(UnitType::Elephant)),
            ),
        ],
    );
}

#[test]
fn test_exhausted_land() {
    test_actions(
        "exhausted_land",
        vec![
            TestAction::not_undoable(
                0,
                Action::StatusPhase(StatusPhaseAction::FreeAdvance(String::from("Storage"))),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("B2"),
                )),
            ),
        ],
    );
}

#[test]
fn test_volcano() {
    test_actions(
        "volcano",
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("C2"),
                )),
            ),
        ],
    );
}

#[test]
fn test_flood() {
    test_actions(
        "flood",
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("C2"),
                )),
            ),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectPosition(
                    Position::from_offset("A1"),
                )),
            ),
        ],
    );
}

#[test]
fn test_earthquake() {
    test_actions(
        "earthquake",
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
                Action::CustomPhaseEvent(CurrentEventResponse::SelectStructures(vec![
                    (Position::from_offset("B2"), Structure::CityCenter),
                    (Position::from_offset("C2"), Structure::Building(Fortress)),
                    (
                        Position::from_offset("C2"),
                        Structure::Wonder("Pyramids".to_string()),
                    ),
                ])),
            ),
            TestAction::not_undoable(
                0,
                Action::CustomPhaseEvent(CurrentEventResponse::Payment(vec![
                    ResourcePile::mood_tokens(1),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::CustomPhaseEvent(CurrentEventResponse::SelectStructures(vec![
                    (Position::from_offset("A1"), Structure::CityCenter),
                    (Position::from_offset("A1"), Structure::Building(Fortress)),
                    (Position::from_offset("A3"), Structure::CityCenter),
                ])),
            ),
        ],
    );
}

#[test]
fn test_migration() {
    test_actions(
        "migration",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
        ],
    );
}

#[test]
fn test_civil_war() {
    test_actions(
        "civil_war",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
        ],
    );
}

