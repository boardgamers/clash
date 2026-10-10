use crate::common::{JsonTest, TestAction, advance_action, move_action, payment_response};
use advance::Advance;
use server::action::Action;
use server::card::HandCard;
use server::city_pieces::Building::Fortress;
use server::content::persistent_events::{EventResponse, SelectedStructure};
use server::leader::Leader;
use server::playing_actions::PlayingAction::Construct;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::status_phase::ChangeGovernment;
use server::structure::Structure;
use server::unit::UnitType;
use server::wonder::Wonder;
use server::{advance, construct};
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
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectUnitType(UnitType::Elephant)),
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
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            )
            .skip_json(),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A3",
                )])),
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
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            ),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![7]))),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A2",
                )])),
            ),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "D2",
                )])),
            ),
            TestAction::not_undoable(0, payment_response(ResourcePile::ore(1))),
        ],
    );
}

#[test]
fn test_barbarians_attack() {
    JSON.test(
        "barbarians_attack",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectUnitType(UnitType::Infantry)),
            ),
        ],
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
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            ),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B2",
                )])),
            ),
        ],
    );
}

const FAMINE: JsonTest = JsonTest::child("incidents", "famine");

#[test]
fn test_pestilence() {
    let cons = Action::Playing(Construct(construct::Construct::new(
        Position::from_offset("C2"),
        Fortress,
        ResourcePile::new(1, 1, 1, 0, 0, 0, 0),
    )));
    FAMINE.test(
        "pestilence",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(0, payment_response(ResourcePile::mood_tokens(1))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A1",
                )])),
            ),
            TestAction::not_undoable(1, payment_response(ResourcePile::mood_tokens(1))),
            TestAction::illegal(0, cons.clone()).skip_json(),
            TestAction::undoable(
                //no compare
                0,
                advance_action(Advance::Sanitation, ResourcePile::gold(2)),
            )
            .skip_json(),
            TestAction::undoable(0, cons).skip_json(),
        ],
    );
}

#[test]
fn test_famine() {
    FAMINE.test(
        "famine",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_famine_protected() {
    FAMINE.test(
        "famine_protected",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_epidemics() {
    FAMINE.test(
        "epidemics",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![7]))),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectUnits(vec![0])))
                .skip_json()
                .with_post_assert(|game| {
                    assert_eq!(game.player(1).units.len(), 1);
                    assert!(game.events.is_empty());
                }),
        ],
    );
}

#[test]
fn epidemics_unit_threshold_and_protection() {
    use server::content::persistent_events::PersistentEventRequest;

    for (unit_count, advance, losses) in [
        (0, None, 0),
        (1, None, 0),
        (1, Some(Advance::Roads), 0),
        (2, None, 1),
        (3, None, 1),
        (2, Some(Advance::Roads), 2),
        (2, Some(Advance::Navigation), 2),
        (2, Some(Advance::TradeRoutes), 2),
        (3, Some(Advance::Roads), 2),
        (2, Some(Advance::Sanitation), 0),
        (3, Some(Advance::Sanitation), 0),
    ] {
        let mut game = FAMINE.load_game("epidemics");
        game.player_mut(0).units.truncate(unit_count);
        game.player_mut(1).units.clear();
        if let Some(advance) = advance {
            game.player_mut(0).advances.insert(advance);
        }
        let mut game = server::game_api::execute(
            game,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
            0,
        );
        if losses > 0 && losses < unit_count {
            let handler = game
                .current_event_handler()
                .expect("Must choose casualties");
            let PersistentEventRequest::SelectUnits(request) = &handler.request else {
                panic!("Expected a casualty selection");
            };
            assert_eq!(request.request.needed, losses as u8..=losses as u8);
            let selected = request.request.choices[..losses].to_vec();
            game = server::game_api::execute(
                game,
                Action::Response(EventResponse::SelectUnits(selected)),
                0,
            );
        }
        assert_eq!(
            game.player(0).units.len(),
            unit_count - losses,
            "Epidemics with {unit_count} units and {advance:?}"
        );
        assert!(game.events.is_empty());
    }
}

const GOOD_YEAR: JsonTest = JsonTest::child("incidents", "good_year");

#[test]
fn test_good_year_with_player_select() {
    GOOD_YEAR.test(
        "good_year",
        vec![
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectUnitType(UnitType::Elephant)),
            ),
        ],
    );
}

#[test]
fn test_successful_year() {
    GOOD_YEAR.test(
        "successful_year",
        vec![TestAction::not_undoable(
            0,
            Action::Response(EventResponse::SelectAdvance(Advance::Storage)),
        )],
    );
}

const EARTHQUAKE: JsonTest = JsonTest::child("incidents", "earthquake");

#[test]
fn test_volcano() {
    EARTHQUAKE.test(
        "volcano",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C2",
                )])),
            ),
        ],
    );
}

#[test]
fn test_flood() {
    EARTHQUAKE.test(
        "flood",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "C2",
                )])),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "A1",
                )])),
            ),
        ],
    );
}

#[test]
fn test_earthquake() {
    EARTHQUAKE.test(
        "earthquake",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2)))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectStructures(vec![
                    SelectedStructure::new(Position::from_offset("B2"), Structure::CityCenter),
                    SelectedStructure::new(
                        Position::from_offset("C2"),
                        Structure::Building(Fortress),
                    ),
                    SelectedStructure::new(
                        Position::from_offset("C2"),
                        Structure::Wonder(Wonder::GreatGardens),
                    ),
                ])),
            )
            .skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::mood_tokens(1))).skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectStructures(vec![
                    SelectedStructure::new(Position::from_offset("A1"), Structure::CityCenter),
                    SelectedStructure::new(
                        Position::from_offset("A1"),
                        Structure::Building(Fortress),
                    ),
                    SelectedStructure::new(Position::from_offset("A3"), Structure::CityCenter),
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
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_civil_war() {
    CIVIL_WAR.test(
        "civil_war",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_revolution() {
    CIVIL_WAR.test(
        "revolution",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2)))
                .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![3])))
                .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![])))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ChangeGovernmentType(ChangeGovernment::new(
                    String::from("Theocracy"),
                    vec![],
                ))),
            ),
        ],
    );
}

#[test]
fn test_uprising() {
    CIVIL_WAR.test(
        "uprising",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(
                0,
                payment_response(ResourcePile::culture_tokens(1) + ResourcePile::mood_tokens(1)),
            ),
        ],
    );
}

#[test]
fn test_envoy() {
    CIVIL_WAR.test(
        "envoy",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(0, advance_action(Advance::Monuments, ResourcePile::gold(2))),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

const TROJAN: JsonTest = JsonTest::child("incidents", "trojan");

#[test]
fn trojan_horse_only_offered_against_another_players_defended_city() {
    use server::content::effects::PermanentEffect;
    use server::events::EventOrigin;

    for (defender, city, fortress, active, expected) in [
        (1, true, false, true, true),
        (1, true, true, true, true),
        (2, true, false, true, false),
        (2, false, false, true, false),
        (1, false, false, true, false),
        (1, true, false, false, false),
    ] {
        let mut game = TROJAN.load_game("trojan_horse");
        let target = Position::from_offset("C1");
        if active {
            game.permanent_effects.push(PermanentEffect::TrojanHorse);
        }
        let mut target_city = game.player_mut(1).cities.remove(0);
        let units = std::mem::take(&mut game.player_mut(1).units);
        if fortress {
            target_city.pieces.fortress = Some(defender);
        } else {
            game.player_mut(defender).units = units;
        }
        if city {
            game.player_mut(defender).cities.push(target_city);
        }
        let game = server::game_api::execute(game, move_action(vec![0, 1, 2, 3], target), 0);
        let offered = game
            .current_event_handler()
            .is_some_and(|handler| handler.origin == EventOrigin::Ability("Trojan Horse".into()));
        assert_eq!(
            offered, expected,
            "defender {defender}, city {city}, fortress {fortress}, active {active}"
        );
        assert_eq!(
            game.permanent_effects
                .iter()
                .any(|effect| matches!(effect, PermanentEffect::TrojanHorse)),
            active
        );
    }
}

#[test]
fn test_trojan_horse() {
    TROJAN.test(
        "trojan_horse",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2)))
                .skip_json(),
            TestAction::undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                payment_response(ResourcePile::culture_tokens(1) + ResourcePile::gold(1)),
            ),
        ],
    );
}

#[test]
fn test_solar_eclipse() {
    TROJAN.test(
        "solar_eclipse",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2)))
                .skip_json(),
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    17,
                )])),
            ),
        ],
    );
}

#[test]
fn test_anarchy() {
    TROJAN.test(
        "anarchy",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(0, advance_action(Advance::Dogma, ResourcePile::gold(2))),
        ],
    );
}

#[test]
fn test_guillotine() {
    TROJAN.test(
        "guillotine",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2)))
                .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectUnitType(UnitType::Leader(
                    Leader::Augustus,
                ))),
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
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_flourishing_trade() {
    TRADE.test(
        "flourishing_trade",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_era_of_stability() {
    TRADE.test(
        "era_of_stability",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            ),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            ),
        ],
    );
}

#[test]
fn test_reformation() {
    TRADE.test(
        "reformation",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::not_undoable(2, Action::Response(EventResponse::SelectPlayer(1))),
        ],
    );
}

const PANDEMICS: JsonTest = JsonTest::child("incidents", "pandemics");

#[test]
fn test_pandemics() {
    PANDEMICS.test(
        "pandemics",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0]))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            ),
            TestAction::not_undoable(1, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn test_black_death() {
    PANDEMICS.test(
        "black_death",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0]))),
        ],
    );
}

#[test]
fn test_vermin() {
    PANDEMICS.test(
        "vermin",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_drought() {
    PANDEMICS.test(
        "drought",
        vec![TestAction::not_undoable(
            0,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
        )],
    );
}

#[test]
fn test_fire() {
    PANDEMICS.test(
        "fire",
        vec![
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::gold(2))),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B2",
                )])),
            ),
        ],
    );
}

#[test]
fn fire_without_forest_cities_loses_wood_and_keeps_mood() {
    use server::log::{ActionLogBalance, ActionLogEntry};
    use server::map::Terrain;
    for wood in [0_u8, 1, 4] {
        let mut game = PANDEMICS.load_game("fire");
        let cities = game
            .player(0)
            .cities
            .iter()
            .map(|c| c.position)
            .collect::<Vec<_>>();
        for position in cities {
            game.map.tiles.insert(position, Terrain::Fertile);
        }
        game.player_mut(0).resources.wood = wood;
        let moods = game
            .players
            .iter()
            .flat_map(|p| p.cities.iter().map(|c| c.mood_state.clone()))
            .collect::<Vec<_>>();
        let gold = game.player(0).resources.gold;
        let game = server::game_api::execute(
            game,
            advance_action(Advance::Storage, ResourcePile::gold(2)),
            0,
        );
        assert_eq!(game.player(0).resources.wood, wood.saturating_sub(1));
        assert_eq!(
            game.player(0).resources.gold,
            gold,
            "Gold deposits restores the 2 gold spent on research"
        );
        assert!(
            game.events.is_empty(),
            "No forest city needs a selection or mood loss"
        );
        assert_eq!(
            game.players
                .iter()
                .flat_map(|p| p.cities.iter().map(|c| c.mood_state.clone()))
                .collect::<Vec<_>>(),
            moods
        );
        let losses = game
            .log
            .iter()
            .flat_map(|a| &a.rounds)
            .flat_map(|r| &r.turns)
            .flat_map(|t| &t.actions)
            .flat_map(|a| &a.items)
            .filter(|item| {
                item.player == 0
                    && item.origin == server::events::EventOrigin::Incident(53)
                    && matches!(
                        &item.entry,
                        ActionLogEntry::Resources {
                            resources,
                            balance: ActionLogBalance::Loss,
                        } if *resources == ResourcePile::wood(1)
                    )
            })
            .count();
        assert_eq!(
            losses,
            usize::from(wood > 0),
            "The journal records the actual wood deduction exactly once"
        );
    }
}

#[test]
fn anarchy_explains_lost_man_god_abilities() {
    use server::cache::Cache;
    use server::game::{Game, GameContext};
    use server::game_data::GameData;
    let mut data = serde_json::to_value(TROJAN.load_game("anarchy").data()).unwrap();
    data["players"][0]["civilization"] = serde_json::json!("Egypt");
    data["players"][0]["advances"] = serde_json::json!([
        "Farming",
        "Mining",
        "Priesthood",
        "StateReligion",
        "Dogma",
        "Conversion",
        "Fanaticism"
    ]);
    let data: GameData = serde_json::from_value(data).unwrap();
    let cache = Cache::new(&data.options);
    let game = Game::from_data(data, cache, GameContext::Play);
    let game = server::game_api::execute(
        game,
        advance_action(Advance::Storage, ResourcePile::gold(2)),
        0,
    );
    let log = serde_json::to_string(&game.log).unwrap();
    assert!(log.contains("Losing Conversion also disables Absolute Power (Man God)"));
    assert!(log.contains("Losing Dogma also disables Nationalism (Man God)"));
    assert!(log.contains("Losing Fanaticism also disables Forced Labor (Man God)"));
    assert!(!log.contains("Losing Devotion"));
    assert!(!game.player(0).can_use_advance(Advance::AbsolutePower));
}
