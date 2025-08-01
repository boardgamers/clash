use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, illegal_action_test, influence_action,
    payment_response,
};
use advance::Advance;
use server::action::{Action, execute_action};
use server::city_pieces::Building::{Academy, Fortress, Temple};
use server::collect::{PositionCollection, possible_resource_collections};
use server::consts::BUILDING_COST;
use server::content::advances::trade_routes::find_trade_routes;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::cultural_influence::InfluenceCultureAttempt;
use server::events::{EventOrigin, check_event_origin};
use server::game::Game;
use server::movement::MovementAction::Move;
use server::movement::{MoveUnits, possible_move_routes};
use server::player::CostTrigger;
use server::playing_actions::PlayingAction::{
    Collect, Construct, Custom, EndTurn, Recruit, WonderCard,
};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::recruit::recruit_cost_without_replaced;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::unit::Units;
use server::wonder::Wonder;
use server::{advance, collect, construct, happiness, recruit};
use std::vec;

mod common;

const JSON: JsonTest = JsonTest::new("advances");

#[test]
fn test_sanitation_and_draft() {
    // we should figure out that sanitation or draft are used, but not both
    // actually that code turns out to be too complicated and slow, so the log now says you
    // used both
    let units = Units::new(1, 1, 0, 0, 0, None);
    let city_position = Position::from_offset("A1");
    JSON.test(
        "sanitation_and_draft",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(recruit::Recruit::new(
                    &units,
                    city_position,
                    ResourcePile::mood_tokens(1) + ResourcePile::gold(2),
                ))),
            )
            .with_pre_assert(move |game| {
                let options = recruit_cost_without_replaced(
                    &game,
                    &game.players[0],
                    &units,
                    city_position,
                    CostTrigger::WithModifiers,
                )
                .unwrap()
                .cost;
                assert_eq!(3, options.conversions.len());
                assert_eq!(ResourcePile::mood_tokens(1), options.conversions[0].to);
                assert_eq!(ResourcePile::mood_tokens(1), options.conversions[1].to);
                assert_eq!(
                    vec![
                        EventOrigin::Advance(Advance::Draft),
                        EventOrigin::Advance(Advance::Sanitation),
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
                advance_action(
                    Advance::SeparationOfPower,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
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
                advance_action(
                    Advance::Devotion,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
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
                advance_action(
                    Advance::Totalitarianism,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
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
                advance_action(
                    Advance::Monuments,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
                0,
            );
        }

        game = execute_action(game, Action::Playing(WonderCard(Wonder::Pyramids)), 0);
        game = execute_action(
            game,
            Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                "C2",
            )])),
            0,
        );
        game = execute_action(
            game,
            payment_response(ResourcePile::new(2, 3, 6, 0, 1, 0, 5)),
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
                Action::Playing(Custom(CustomAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
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
                Action::Playing(Custom(CustomAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(2))),
        ],
    );
}

#[test]
fn test_increase_happiness_voting() {
    JSON.test(
        "increase_happiness_voting",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::IncreaseHappiness(
                happiness::IncreaseHappiness::new(
                    vec![
                        (Position::from_offset("C2"), 1),
                        (Position::from_offset("B3"), 2),
                    ],
                    ResourcePile::mood_tokens(6),
                    PlayingActionType::Custom(CustomActionType::VotingIncreaseHappiness),
                ),
            )),
        )],
    );
}

#[test]
fn test_increase_happiness_voting_rituals() {
    JSON.test(
        "increase_happiness_voting_rituals",
        vec![TestAction::undoable(
            0,
            Action::Playing(PlayingAction::IncreaseHappiness(
                happiness::IncreaseHappiness::new(
                    vec![
                        (Position::from_offset("C2"), 1),
                        (Position::from_offset("B3"), 2),
                    ],
                    ResourcePile::new(1, 0, 1, 3, 1, 0, 0),
                    PlayingActionType::Custom(CustomActionType::VotingIncreaseHappiness),
                ),
            )),
        )],
    );
}

#[test]
fn test_absolute_power() {
    JSON.test(
        "absolute_power",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::AbsolutePower)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::mood_tokens(2))),
        ],
    );
}

#[test]
fn test_forced_labor() {
    JSON.test(
        "forced_labor",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::ForcedLabor)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::mood_tokens(1))).skip_json(),
            TestAction::undoable(
                0,
                Action::Playing(Collect(collect::Collect::new(
                    Position::from_offset("A1"),
                    vec![
                        PositionCollection::new(Position::from_offset("A1"), ResourcePile::food(1)),
                        PositionCollection::new(Position::from_offset("A2"), ResourcePile::wood(1)),
                    ],
                    PlayingActionType::Collect,
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
            TestAction::undoable(0, custom_action(CustomActionType::CivilLiberties)),
            TestAction::undoable(
                0,
                Action::Playing(Recruit(recruit::Recruit::new(
                    &Units::new(0, 1, 0, 0, 0, None),
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
            TestAction::undoable(0, custom_action(CustomActionType::Bartering)).skip_json(),
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
            Action::Movement(Move(MoveUnits::new(
                units,
                destination,
                None,
                ResourcePile::food(1) + ResourcePile::ore(1),
            ))),
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
            Action::Movement(Move(MoveUnits::new(
                units,
                destination,
                None,
                ResourcePile::food(1) + ResourcePile::ore(1),
            ))),
        )],
    );
}

#[test]
fn test_road_coordinates() {
    let game = &JSON.load_game("roads_unit_test");
    // city units at D8 are 0, 1, 2

    // 3 and 4 are on mountain C8 and can move to the city D8 (ignoring movement restrictions),
    // but not both, since the city already has 3 army units
    assert!(get_destinations(game, &[4], "C8").contains(&"D8".to_string()));
    assert!(!get_destinations(game, &[3, 4], "C8").contains(&"D8".to_string()));

    // 5 and 6 are on E8 and count against the stack size limit of the units moving out of city D8
    // so only 2 can move over them towards F7
    assert!(get_destinations(game, &[0, 1], "D8").contains(&"F7".to_string()));
    let city_dest = get_destinations(game, &[0, 1, 2], "D8");
    // all 3 units could move around E8 if we had roman roads
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
    possible_move_routes(player, game, units, Position::from_offset(position), None)
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
            TestAction::undoable(0, custom_action(CustomActionType::Theaters)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn test_taxes() {
    JSON.test(
        "taxes",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Taxes)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::mood_tokens(1))).skip_json(),
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
fn test_dogma_with_anarchy() {
    JSON.test(
        "dogma",
        vec![
            TestAction::undoable(1, advance_action(Advance::Dogma, ResourcePile::ideas(2)))
                .skip_json(),
            TestAction::undoable(
                1,
                Action::Playing(Construct(construct::Construct::new(
                    Position::from_offset("C1"),
                    Temple,
                    ResourcePile::new(0, 1, 1, 0, 0, 0, 0),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectAdvance(Advance::Fanaticism)),
            ),
        ],
    );
}

#[test]
fn test_priesthood() {
    JSON.test(
        "priesthood",
        vec![
            TestAction::undoable(1, advance_action(Advance::Math, ResourcePile::empty())),
            TestAction::undoable(1, advance_action(Advance::Astronomy, ResourcePile::gold(2))),
            TestAction::illegal(1, advance_action(Advance::Astronomy, ResourcePile::empty())),
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
                advance_action(
                    Advance::Writing,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
            )
            .skip_json(),
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
                advance_action(
                    Advance::Draft,
                    ResourcePile::food(1) + ResourcePile::gold(1),
                ),
            ),
            TestAction::undoable(0, payment_response(ResourcePile::ideas(1))),
        ],
    );
}

#[test]
fn test_collect_fishing() {
    JSON.test(
        "collect_fishing",
        vec![TestAction::undoable(
            0,
            Action::Playing(Collect(collect::Collect::new(
                Position::from_offset("C2"),
                vec![PositionCollection::new(
                    Position::from_offset("C3"),
                    ResourcePile::food(1),
                )],
                PlayingActionType::Collect,
            ))),
        )],
    );
}

#[test]
fn test_collect_port() {
    JSON.test(
        "collect_port",
        vec![TestAction::undoable(
            0,
            Action::Playing(Collect(collect::Collect::new(
                Position::from_offset("C2"),
                vec![PositionCollection::new(
                    Position::from_offset("C3"),
                    ResourcePile::mood_tokens(1),
                )],
                PlayingActionType::Collect,
            ))),
        )],
    );
}

#[test]
fn test_collect_husbandry() {
    let action = Action::Playing(Collect(collect::Collect::new(
        Position::from_offset("B3"),
        vec![PositionCollection::new(
            Position::from_offset("B5"),
            ResourcePile::food(1),
        )],
        PlayingActionType::Collect,
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
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Collect(collect::Collect::new(
                    Position::from_offset("C2"),
                    vec![
                        PositionCollection::new(Position::from_offset("B1"), ResourcePile::ore(1)),
                        PositionCollection::new(Position::from_offset("B2"), ResourcePile::ore(1)),
                    ],
                    PlayingActionType::Custom(CustomActionType::FreeEconomyCollect),
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::mood_tokens(1)))
                .with_post_assert(|game| {
                    // no production focus
                    let result =
                        PlayingActionType::ActionCard(19).is_available(&game, game.active_player());
                    assert!(result.is_err());
                }),
        ],
    );
}

#[test]
fn test_cultural_influence_instant_with_arts() {
    JSON.test(
        "cultural_influence_instant_with_arts",
        vec![
            TestAction::undoable(
                1,
                Action::Playing(PlayingAction::InfluenceCultureAttempt(
                    InfluenceCultureAttempt::new(
                        SelectedStructure::new(
                            Position::from_offset("C2"),
                            Structure::Building(Fortress),
                        ),
                        PlayingActionType::Custom(CustomActionType::ArtsInfluenceCultureAttempt),
                    ),
                )),
            )
            .skip_json(),
            TestAction::not_undoable(1, payment_response(ResourcePile::culture_tokens(1))),
        ],
    )
}

#[test]
fn test_cultural_influence_with_conversion() {
    JSON.test(
        "cultural_influence_with_conversion",
        vec![
            TestAction::undoable(1, influence_action()).skip_json(),
            TestAction::not_undoable(1, payment_response(ResourcePile::culture_tokens(1)))
                .skip_json(),
            TestAction::undoable(1, payment_response(ResourcePile::culture_tokens(3))),
        ],
    );
}

#[test]
fn test_overpay() {
    JSON.test(
        "sanitation_and_draft",
        vec![TestAction::illegal(
            0,
            Action::Playing(Recruit(recruit::Recruit::new(
                &Units::new(0, 1, 0, 0, 0, None),
                Position::from_offset("A1"),
                ResourcePile::mood_tokens(1) + ResourcePile::gold(2), //paid too much
            ))),
        )],
    );
}

#[test]
fn test_husbandry() {
    JSON.test(
        "husbandry",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Collect(collect::Collect::new(
                    Position::from_offset("C2"),
                    vec![PositionCollection::new(
                        Position::from_offset("D1"),
                        ResourcePile::food(1),
                    )],
                    PlayingActionType::Collect,
                ))),
            )
            .skip_json()
            .with_post_assert(|game| {
                // but not again
                assert!(has_husbandry_field(&game))
            }),
            // can use husbandry - because it was not used in the previous action
            TestAction::undoable(
                0,
                Action::Playing(Collect(collect::Collect::new(
                    Position::from_offset("C2"),
                    vec![PositionCollection::new(
                        Position::from_offset("E2"),
                        ResourcePile::food(1),
                    )],
                    PlayingActionType::Collect,
                ))),
            )
            .skip_json()
            .with_post_assert(|game| {
                // but not again
                assert!(!has_husbandry_field(&game))
            }),
        ],
    );
}

fn has_husbandry_field(game: &Game) -> bool {
    let info = possible_resource_collections(
        game,
        Position::from_offset("C2"),
        0,
        &check_event_origin(),
        CostTrigger::NoModifiers,
    );
    info.choices.contains_key(&Position::from_offset("E2"))
}
