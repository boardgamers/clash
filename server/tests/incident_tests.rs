use crate::common::{move_action, JsonTest, TestAction};
use server::action::Action;
use server::city_pieces::Building::Fortress;
use server::content::custom_phase_actions::{CurrentEventResponse, Structure};
use server::playing_actions;
use server::playing_actions::PlayingAction::{Advance, Construct};
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::{ChangeGovernment, ChangeGovernmentType};
use server::unit::UnitType;
use std::vec;

mod common;

const JSON: JsonTest = JsonTest::new("incidents");

#[test]
fn test_barbarians_spawn() {
    JSON.test(
        "barbarians_spawn",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("B3"),
                ])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectUnitType(UnitType::Elephant)),
            ),
        ],
    );
}

#[test]
fn test_barbarians_move() {
    JSON.test(
        "barbarians_move",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("B3"),
                ])),
            ),
        ],
    );
}

#[test]
fn test_pirates_spawn() {
    JSON.test(
        "pirates_spawn",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectUnits(vec![7])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("A2"),
                ])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("D2"),
                ])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::Payment(vec![ResourcePile::ore(1)])),
            ),
        ],
    );
}

#[test]
fn test_barbarians_attack() {
    JSON.test(
        "barbarians_attack",
        vec![TestAction::not_undoable(
            0,
            Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
        )],
    );
}

#[test]
fn test_barbarians_recapture_city() {
    JSON.test(
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
    JSON.test(
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
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("A1"),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::Payment(vec![
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
    JSON.test(
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
    JSON.test(
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
                Action::Response(CurrentEventResponse::SelectUnits(vec![7])),
            ),
        ],
    );
}

#[test]
fn test_good_year_with_player_select() {
    JSON.test(
        "good_year",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectUnitType(UnitType::Elephant)),
            ),
        ],
    );
}

#[test]
fn test_exhausted_land() {
    JSON.test(
        "exhausted_land",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectAdvance("Storage".to_string())),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("B2"),
                ])),
            ),
        ],
    );
}

#[test]
fn test_volcano() {
    JSON.test(
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
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("C2"),
                ])),
            ),
        ],
    );
}

#[test]
fn test_flood() {
    JSON.test(
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
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("C2"),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::SelectPositions(vec![
                    Position::from_offset("A1"),
                ])),
            ),
        ],
    );
}

#[test]
fn test_earthquake() {
    JSON.test(
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
                Action::Response(CurrentEventResponse::SelectStructures(vec![
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
                Action::Response(CurrentEventResponse::Payment(vec![
                    ResourcePile::mood_tokens(1),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::SelectStructures(vec![
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
    JSON.test(
        "migration",
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
fn test_civil_war() {
    JSON.test(
        "civil_war",
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
fn test_revolution() {
    JSON.test(
        "revolution",
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
                Action::Response(CurrentEventResponse::SelectUnits(vec![3])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectUnits(vec![])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::ChangeGovernmentType(
                    ChangeGovernmentType::ChangeGovernment(ChangeGovernment {
                        new_government: String::from("Theocracy"),
                        additional_advances: vec![],
                    }),
                )),
            ),
        ],
    );
}

#[test]
fn test_uprising() {
    JSON.test(
        "uprising",
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
                Action::Response(CurrentEventResponse::Payment(vec![
                    ResourcePile::mood_tokens(1) + ResourcePile::culture_tokens(1),
                ])),
            ),
        ],
    );
}

#[test]
fn test_envoy() {
    JSON.test(
        "envoy",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Monuments"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::undoable(0, Action::Response(CurrentEventResponse::Bool(true))),
        ],
    );
}

#[test]
fn test_trojan_horse() {
    JSON.test(
        "trojan_horse",
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
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::Payment(vec![
                    ResourcePile::culture_tokens(1) + ResourcePile::gold(1),
                ])),
            ),
        ],
    );
}

#[test]
fn test_solar_eclipse() {
    JSON.test(
        "solar_eclipse",
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
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            ),
        ],
    );
}

#[test]
fn test_anarchy() {
    JSON.test(
        "anarchy",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Dogma"),
                    payment: ResourcePile::gold(2),
                }),
            ),
        ],
    );
}

#[test]
fn test_scientific_trade() {
    JSON.test(
        "scientific_trade",
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
fn test_flourishing_trade() {
    JSON.test(
        "flourishing_trade",
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
fn test_era_of_stability() {
    JSON.test(
        "era_of_stability",
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
                Action::Response(CurrentEventResponse::ResourceReward(
                    ResourcePile::culture_tokens(1),
                )),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::ResourceReward(
                    ResourcePile::culture_tokens(1),
                )),
            ),
        ],
    );
}

#[test]
fn test_reformation() {
    JSON.test(
        "reformation",
        vec![TestAction::not_undoable(
            0,
            Action::Playing(Advance {
                advance: String::from("Storage"),
                payment: ResourcePile::gold(2),
            }),
        ),
        TestAction::not_undoable(
            2,
            Action::Response(CurrentEventResponse::SelectPlayer(1))
        )],
    );
}
