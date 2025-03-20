use crate::common::{move_action, JsonTest, TestAction};
use server::action::Action;
use server::card::HandCard;
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

const FAMINE: JsonTest = JsonTest::child("incidents", "famine");

#[test]
fn test_pestilence() {
    let cons = Action::Playing(Construct(playing_actions::Construct {
        city_position: Position::from_offset("C2"),
        city_piece: Fortress,
        payment: ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
        port_position: None,
    }));
    FAMINE.test(
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
                Action::Response(CurrentEventResponse::Payment(vec![
                    ResourcePile::mood_tokens(1),
                ])),
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
    FAMINE.test(
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
    FAMINE.test(
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

const GOOD_YEAR: JsonTest = JsonTest::child("incidents", "good_year");

#[test]
fn test_good_year_with_player_select() {
    GOOD_YEAR.test(
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

const EARTHQUAKE: JsonTest = JsonTest::child("incidents", "earthquake");

#[test]
fn test_volcano() {
    EARTHQUAKE.test(
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
    EARTHQUAKE.test(
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
    EARTHQUAKE.test(
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

const CIVIL_WAR: JsonTest = JsonTest::child("incidents", "civil_war");

#[test]
fn test_migration() {
    CIVIL_WAR.test(
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
    CIVIL_WAR.test(
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
    CIVIL_WAR.test(
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
    CIVIL_WAR.test(
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
    CIVIL_WAR.test(
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

const TROJAN: JsonTest = JsonTest::child("incidents", "trojan");

#[test]
fn test_trojan_horse() {
    TROJAN.test(
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
    TROJAN.test(
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
    TROJAN.test(
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

const TRADE: JsonTest = JsonTest::child("incidents", "trade");

#[test]
fn test_scientific_trade() {
    TRADE.test(
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
    TRADE.test(
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
    TRADE.test(
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
    TRADE.test(
        "reformation",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Storage"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::not_undoable(2, Action::Response(CurrentEventResponse::SelectPlayer(1))),
        ],
    );
}

const PANDEMICS: JsonTest = JsonTest::child("incidents", "pandemics");

#[test]
fn test_pandemics() {
    PANDEMICS.test(
        "pandemics",
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
                Action::Response(CurrentEventResponse::SelectUnits(vec![0])),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(CurrentEventResponse::SelectHandCards(vec![
                    HandCard::ActionCard(1),
                ])),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(CurrentEventResponse::Payment(vec![
                    ResourcePile::culture_tokens(1),
                ])),
            ),
        ],
    );
}

#[test]
fn test_black_death() {
    PANDEMICS.test(
        "black_death",
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
                Action::Response(CurrentEventResponse::SelectUnits(vec![0])),
            ),
        ],
    );
}

#[test]
fn test_vermin() {
    PANDEMICS.test(
        "vermin",
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
fn test_drought() {
    PANDEMICS.test(
        "drought",
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
fn test_fire() {
    PANDEMICS.test(
        "fire",
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
                    Position::from_offset("B2"),
                ])),
            ),
        ],
    );
}
