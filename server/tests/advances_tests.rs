use crate::common::{JsonTest, TestAction, illegal_action_test, influence_action};
use server::action::{Action, execute_action};
use server::city_pieces::Building::{Academy, Fortress, Temple};
use server::collect::PositionCollection;
use server::consts::BUILDING_COST;
use server::content::advances::trade_routes::find_trade_routes;
use server::content::custom_actions::CustomAction::{
    ArtsInfluenceCultureAttempt, VotingIncreaseHappiness,
};
use server::content::custom_actions::{CustomAction, CustomActionType, CustomEventAction};
use server::content::persistent_events::{EventResponse, SelectedStructure, Structure};
use server::events::EventOrigin;
use server::game::Game;
use server::movement::MovementAction::Move;
use server::movement::{MoveUnits, move_units_destinations};
use server::playing_actions::PlayingAction::{
    Advance, Collect, Construct, Custom, CustomEvent, EndTurn, Recruit, WonderCard,
};
use server::position::Position;
use server::recruit::recruit_cost_without_replaced;
use server::resource_pile::ResourcePile;
use server::unit::Units;
use server::{construct, playing_actions};
use std::vec;

mod common;

const JSON: JsonTest = JsonTest::new("advances");

#[test]
fn test_sanitation_and_draft() {
    // we should figure out that sanitation or draft are used, but not both
    let units = Units::new(1, 1, 0, 0, 0, 0);
    let city_position = Position::from_offset("A1");
    JSON.test(
        "sanitation_and_draft",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::playing_actions::Recruit::new(
                    &units,
                    city_position,
                    ResourcePile::mood_tokens(1) + ResourcePile::gold(2),
                ))),
            )
            .with_pre_assert(move |game| {
                let options = recruit_cost_without_replaced(
                    &game.players[0],
                    &units,
                    city_position,
                    None,
                    None,
                )
                .unwrap()
                .cost;
                assert_eq!(3, options.conversions.len());
                assert_eq!(ResourcePile::mood_tokens(1), options.conversions[0].to);
                assert_eq!(ResourcePile::mood_tokens(1), options.conversions[1].to);
                assert_eq!(
                    vec![
                        EventOrigin::Advance("Sanitation".to_string()),
                        EventOrigin::Advance("Draft".to_string())
                    ],
                    options.modifiers
                );
            }),
        ],
    );
}

#[test]
fn test_separation_of_power() {
    illegal_action_test(|test| {
        let mut game = load_culture();
        game = execute_action(game, Action::Playing(EndTurn), 1);
        if test.fail {
            game = execute_action(
                game,
                Action::Playing(Advance {
                    advance: String::from("Separation of Power"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game = execute_action(game, Action::Playing(EndTurn), 0);
        test.setup_done = true;
        let _ = execute_action(game, influence_action(), 1);
    });
}

#[test]
fn test_devotion() {
    illegal_action_test(|test| {
        let mut game = load_culture();
        game = execute_action(game, Action::Playing(EndTurn), 1);
        if test.fail {
            game = execute_action(
                game,
                Action::Playing(Advance {
                    advance: String::from("Devotion"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game = execute_action(game, Action::Playing(EndTurn), 0);
        test.setup_done = true;
        let _ = execute_action(game, influence_action(), 1);
    });
}

fn load_culture() -> Game {
    JsonTest::new("base").load_game("cultural_influence")
}

#[test]
fn test_totalitarianism() {
    illegal_action_test(|test| {
        let mut game = load_culture();
        game = execute_action(game, Action::Playing(EndTurn), 1);
        if test.fail {
            game = execute_action(
                game,
                Action::Playing(Advance {
                    advance: String::from("Totalitarianism"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }
        game = execute_action(game, Action::Playing(EndTurn), 0);
        test.setup_done = true;
        let _ = execute_action(game, influence_action(), 1);
    });
}

#[test]
fn test_monuments() {
    illegal_action_test(|test| {
        let mut game = load_culture();
        game = execute_action(game, Action::Playing(EndTurn), 1);
        if test.fail {
            game = execute_action(
                game,
                Action::Playing(Advance {
                    advance: String::from("Monuments"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
                0,
            );
        }

        game = execute_action(game, Action::Playing(WonderCard("Pyramids".to_string())), 0);
        game = execute_action(
            game,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "C2",
            )])),
            0,
        );
        game = execute_action(
            game,
            Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                2, 3, 3, 0, 0, 0, 4,
            )])),
            0,
        );
        game = execute_action(game, Action::Playing(EndTurn), 0);
        test.setup_done = true;
        let _ = execute_action(game, influence_action(), 1);
    });
}

#[test]
fn test_increase_happiness_sports() {
    JSON.test(
        "increase_happiness_sports",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_increase_happiness_sports2() {
    JSON.test(
        "increase_happiness_sports2",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    2,
                )])),
            ),
        ],
    );
}

#[test]
fn test_increase_happiness_voting() {
    JSON.test(
        "increase_happiness_voting",
        vec![TestAction::undoable(
            0,
            Action::Playing(Custom(VotingIncreaseHappiness(
                playing_actions::IncreaseHappiness {
                    happiness_increases: vec![
                        (Position::from_offset("C2"), 1),
                        (Position::from_offset("B3"), 2),
                    ],
                    payment: ResourcePile::mood_tokens(5),
                },
            ))),
        )],
    );
}

#[test]
fn test_increase_happiness_voting_rituals() {
    JSON.test(
        "increase_happiness_voting_rituals",
        vec![TestAction::undoable(
            0,
            Action::Playing(Custom(VotingIncreaseHappiness(
                playing_actions::IncreaseHappiness {
                    happiness_increases: vec![
                        (Position::from_offset("C2"), 1),
                        (Position::from_offset("B3"), 2),
                    ],
                    payment: ResourcePile::new(1, 0, 1, 1, 1, 1, 0),
                },
            ))),
        )],
    );
}

#[test]
fn test_absolute_power() {
    JSON.test(
        "absolute_power",
        vec![TestAction::undoable(
            0,
            Action::Playing(CustomEvent(CustomEventAction::new(
                CustomActionType::AbsolutePower,
                None,
            ))),
        )],
    );
}

#[test]
fn test_forced_labor() {
    JSON.test(
        "forced_labor",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::ForcedLabor,
                    None,
                ))),
            ),
            TestAction::undoable(
                0,
                Action::Playing(Collect(playing_actions::Collect::new(
                    Position::from_offset("A1"),
                    vec![
                        PositionCollection::new(Position::from_offset("A1"), ResourcePile::food(1)),
                        PositionCollection::new(Position::from_offset("A2"), ResourcePile::wood(1)),
                    ],
                    ResourcePile::food(1) + ResourcePile::wood(1),
                ))),
            ),
        ],
    );
}

#[test]
fn test_civil_liberties() {
    JSON.test(
        "civil_liberties",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::CivilLiberties,
                    None,
                ))),
            ),
            TestAction::undoable(
                0,
                Action::Playing(Recruit(playing_actions::Recruit::new(
                    &Units::new(0, 1, 0, 0, 0, 0),
                    Position::from_offset("A1"),
                    ResourcePile::mood_tokens(2),
                ))),
            ),
        ],
    );
}

#[test]
fn test_bartering() {
    JSON.test(
        "bartering",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::Bartering,
                    None,
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::gold(1))),
            ),
        ],
    );
}

#[test]
fn test_movement_on_roads_from_city() {
    let units = vec![0];
    let destination = Position::from_offset("F7");
    JSON.test(
        "movement_on_roads_from_city",
        vec![TestAction::undoable(
            1,
            Action::Movement(Move(MoveUnits {
                units,
                destination,
                embark_carrier_id: None,
                payment: ResourcePile::food(1) + ResourcePile::ore(1),
            })),
        )],
    );
}

#[test]
fn test_movement_on_roads_to_city() {
    let units = vec![0];
    let destination = Position::from_offset("D8");
    JSON.test(
        "movement_on_roads_to_city",
        vec![TestAction::undoable(
            1,
            Action::Movement(Move(MoveUnits {
                units,
                destination,
                embark_carrier_id: None,
                payment: ResourcePile::food(1) + ResourcePile::ore(1),
            })),
        )],
    );
}

#[test]
fn test_road_coordinates() {
    let game = &JSON.load_game("roads_unit_test");
    // city units at D8 are 0, 1, 2

    // 3 and 4 are on mountain C8 and can move to the city at D8 (ignoring movement restrictions),
    // but not both, since the city already has 3 army units
    assert!(get_destinations(game, &[4], "C8").contains(&"D8".to_string()));
    assert!(!get_destinations(game, &[3, 4], "C8").contains(&"D8".to_string()));

    // 5 and 6 are on E8 and count against the stack size limit of the units moving out of city D8
    // so only 2 can move over them towards F7
    assert!(get_destinations(game, &[0, 1], "D8").contains(&"F7".to_string()));
    let city_dest = get_destinations(game, &[0, 1, 2], "D8");
    assert!(!city_dest.contains(&"F7".to_string()));

    // all 3 city units can move around the mountain to C7
    assert!(city_dest.contains(&"C7".to_string()));
    // explore for the city units at D6 is not allowed
    assert!(!city_dest.contains(&"D6".to_string()));
    // embark for the city units at E7 is not allowed
    assert!(!city_dest.contains(&"E7".to_string()));

    // don't move to same position
    assert!(!city_dest.contains(&"D8".to_string()));
}

fn get_destinations(game: &Game, units: &[u32], position: &str) -> Vec<String> {
    let player = game.player(1);
    move_units_destinations(player, game, units, Position::from_offset(position), None)
        .unwrap()
        .into_iter()
        .map(|r| r.destination.to_string())
        .collect()
}

#[test]
fn test_theaters() {
    JSON.test(
        "theaters",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::Theaters,
                    None,
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_taxes() {
    JSON.test(
        "taxes",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(CustomEvent(CustomEventAction::new(
                    CustomActionType::Taxes,
                    None,
                ))),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::new(
                    1, 1, 1, 0, 1, 0, 0,
                ))),
            ),
        ],
    );
}

#[test]
fn test_trade_route_coordinates() {
    let game = &JSON.load_game("trade_routes_unit_test");
    // trading cities are C6, D6, E6, B6

    // 1 ships and 1 settler on E7 can trade with C6, D6, E6

    // can't trade:
    // 1 settler is at C8, but the path is not explored (or blocked by a pirate at C7)
    // 1 ship is at A7, but the pirate at A8 blocks trading in its zone of control

    let found = find_trade_routes(game, game.player(1), false);
    assert_eq!(found.len(), 2);
}

#[test]
fn test_trade_routes() {
    JSON.test(
        "trade_routes",
        vec![TestAction::not_undoable(0, Action::Playing(EndTurn))],
    );
}

#[test]
fn test_trade_routes_with_currency() {
    JSON.test(
        "trade_routes_with_currency",
        vec![
            TestAction::not_undoable(0, Action::Playing(EndTurn)),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::ResourceReward(
                    ResourcePile::gold(1) + ResourcePile::food(1),
                )),
            ),
        ],
    );
}

#[test]
fn test_dogma() {
    JSON.test(
        "dogma",
        vec![
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Dogma"),
                    payment: ResourcePile::ideas(2),
                }),
            ),
            TestAction::undoable(
                1,
                Action::Playing(Construct(construct::Construct::new(
                    Position::from_offset("C1"),
                    Temple,
                    ResourcePile::new(0, 1, 1, 0, 0, 0, 0),
                ))),
            ),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            ),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectAdvance("Fanaticism".to_string())),
            ),
        ],
    );
}

#[test]
fn test_priesthood() {
    JSON.test(
        "priesthood",
        vec![
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Math"),
                    payment: ResourcePile::empty(),
                }),
            ),
            TestAction::undoable(
                1,
                Action::Playing(Advance {
                    advance: String::from("Astronomy"),
                    payment: ResourcePile::gold(2),
                }),
            ),
            TestAction::illegal(
                1,
                Action::Playing(Advance {
                    advance: String::from("Astronomy"),
                    payment: ResourcePile::empty(),
                }),
            ),
        ],
    );
}

#[test]
fn test_writing() {
    JSON.test(
        "writing",
        vec![
            TestAction::not_undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Writing"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
            )
            .without_json_comparison(),
            TestAction::undoable(
                0,
                Action::Playing(Construct(construct::Construct::new(
                    Position::from_offset("A1"),
                    Academy,
                    BUILDING_COST.clone(),
                ))),
            ),
        ],
    );
}
#[test]
fn test_free_education() {
    JSON.test(
        "free_education",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Advance {
                    advance: String::from("Draft"),
                    payment: ResourcePile::food(1) + ResourcePile::gold(1),
                }),
            ),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::Payment(vec![ResourcePile::ideas(1)])),
            ),
        ],
    );
}

#[test]
fn test_collect_husbandry() {
    let action = Action::Playing(Collect(playing_actions::Collect::new(
        Position::from_offset("B3"),
        vec![PositionCollection::new(
            Position::from_offset("B5"),
            ResourcePile::food(1),
        )],
        ResourcePile::food(1),
    )));
    JSON.test(
        "collect_husbandry",
        vec![
            TestAction::undoable(0, action.clone()),
            TestAction::illegal(0, action.clone()), // illegal because it can't be done again
        ],
    );
}

#[test]
fn test_collect_free_economy() {
    JSON.test(
        "collect_free_economy",
        vec![TestAction::undoable(
            0,
            Action::Playing(Custom(CustomAction::FreeEconomyCollect(
                playing_actions::Collect::new(
                    Position::from_offset("C2"),
                    vec![
                        PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1)),
                        PositionCollection::new(Position::from_offset("B2"), ResourcePile::ore(1)),
                    ],
                    ResourcePile::ore(2),
                ),
            ))),
        )],
    );
}

#[test]
fn test_cultural_influence_instant_with_arts() {
    JSON.test(
        "cultural_influence_instant_with_arts",
        vec![TestAction::not_undoable(
            1,
            Action::Playing(Custom(ArtsInfluenceCultureAttempt(SelectedStructure::new(
                Position::from_offset("C2"),
                Structure::Building(Fortress),
            )))),
        )],
    )
}

#[test]
fn test_cultural_influence_with_conversion() {
    JSON.test(
        "cultural_influence_with_conversion",
        vec![
            TestAction::not_undoable(1, influence_action()),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::Payment(vec![ResourcePile::culture_tokens(
                    3,
                )])),
            ),
        ],
    );
}

#[test]
fn test_overpay() {
    JSON.test(
        "sanitation_and_draft",
        vec![TestAction::illegal(
            0,
            Action::Playing(Recruit(server::playing_actions::Recruit::new(
                &Units::new(0, 1, 0, 0, 0, 0),
                Position::from_offset("A1"),
                ResourcePile::mood_tokens(1) + ResourcePile::gold(2), //paid too much
            ))),
        )],
    );
}
