use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::game::Game;
use server::movement::MoveUnits;
use server::movement::MovementAction::Move;
use server::movement::possible_move_routes;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::set_unit_position;

mod common;

const JSON: JsonTest = JsonTest::new("movement");

#[test]
fn test_movement() {
    JSON.test(
        "movement",
        vec![TestAction::undoable(
            0,
            move_action(vec![4], Position::from_offset("B3")),
        )],
    );
}

#[test]
fn test_explore_choose() {
    JSON.test(
        "explore_choose",
        vec![TestAction::not_undoable(
            1,
            move_action(vec![0], Position::from_offset("C7")),
        )],
    );
}

#[test]
fn test_explore_auto_no_walk_on_water() {
    JSON.test(
        "explore_auto_no_walk_on_water",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("B2")),
        )],
    );
}

#[test]
fn test_explore_auto_adjacent_water() {
    JSON.test(
        "explore_auto_adjacent_water",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C7")),
        )],
    );
}

#[test]
fn test_explore_auto_water_outside() {
    JSON.test(
        "explore_auto_water_outside",
        vec![TestAction::not_undoable(
            1,
            move_action(vec![1], Position::from_offset("F5")),
        )],
    );
}

#[test]
fn test_explore_resolution() {
    JSON.test(
        "explore_resolution",
        vec![
            TestAction::not_undoable(1, move_action(vec![0], Position::from_offset("D6"))),
            TestAction::undoable(1, Action::Response(EventResponse::ExploreResolution(3))),
        ],
    );
}

#[test]
fn test_ship_transport() {
    JSON.test(
        "ship_transport",
        vec![TestAction::undoable(
            0,
            move_action(vec![7], Position::from_offset("D2")),
        )],
    );
}

#[test]
fn test_ship_transport_same_sea() {
    JSON.test(
        "ship_transport_same_sea",
        vec![TestAction::undoable(
            0,
            move_action(vec![7], Position::from_offset("C3")),
        )],
    );
}

#[test]
fn test_ship_embark() {
    JSON.test(
        "ship_embark",
        vec![TestAction::undoable(
            0,
            Action::Movement(Move(MoveUnits::new(
                vec![3, 4],
                Position::from_offset("C3"),
                Some(8),
                ResourcePile::empty(),
            ))),
        )],
    );
}

#[test]
fn test_ship_embark_continue() {
    JSON.test(
        "ship_embark_continue",
        vec![TestAction::undoable(
            0,
            Action::Movement(Move(MoveUnits::new(
                vec![5, 6],
                Position::from_offset("C3"),
                Some(9),
                ResourcePile::empty(),
            ))),
        )],
    );
}

#[test]
fn test_ship_disembark() {
    JSON.test(
        "ship_disembark",
        vec![TestAction::undoable(
            0,
            move_action(vec![1, 2], Position::from_offset("B3")),
        )],
    );
}

#[test]
fn test_ship_disembark_capture_empty_city() {
    JSON.test(
        "ship_disembark_capture_empty_city",
        vec![
            TestAction::undoable(0, move_action(vec![1, 2], Position::from_offset("B2")))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(10),
                ])),
            ),
        ],
    );
}

#[test]
fn test_ship_explore() {
    JSON.test(
        "ship_explore",
        vec![
            TestAction::not_undoable(1, move_action(vec![1], Position::from_offset("C5"))),
            TestAction::undoable(
                1,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "B5",
                )])),
            )
            .skip_json()
            .with_post_assert(|game| {
                assert_eq!(
                    game.player(1).get_unit(1).position,
                    Position::from_offset("B5")
                );
                assert!(game.current_event_handler().is_none());
            }),
        ],
    );
}

#[test]
fn test_ship_explore_teleport() {
    JSON.test(
        "ship_explore_teleport",
        vec![TestAction::not_undoable(
            1,
            move_action(vec![1], Position::from_offset("C4")),
        )],
    );
}

#[test]
fn test_ship_explore_move_not_possible() {
    JSON.test(
        "ship_explore_move_not_possible",
        vec![TestAction::undoable(
            1,
            Action::Response(EventResponse::ExploreResolution(3)),
        )],
    );
}

#[test]
fn test_ship_navigate() {
    JSON.test(
        "ship_navigate",
        vec![TestAction::undoable(
            1,
            move_action(vec![1], Position::from_offset("A7")),
        )],
    );
}

#[test]
fn test_ship_navigate_coordinates() {
    let mut game = JSON.load_game("ship_navigation_unit_test");

    let pairs = [
        ("B3", "B5", true),
        ("B5", "A7", true),
        ("A7", "F7", true),
        ("F7", "G3", false),
        ("G7", "A7", false),
        ("G7", "G3", true),
        ("G3", "B3", true),
    ];

    for (from, to, back) in pairs {
        let from = Position::from_offset(from);
        let to = Position::from_offset(to);
        assert_navigate(&mut game, from, to);
        if back {
            assert_navigate(&mut game, to, from);
        }
    }
}

fn assert_navigate(game: &mut Game, from: Position, to: Position) {
    set_unit_position(1, 1, from, game);
    let result = possible_move_routes(game.player(1), game, &[1], from, None)
        .is_ok_and(|d| d.iter().any(|route| route.destination == to));
    assert!(result, "expected to be able to move from {from} to {to}");
}

#[test]
fn test_ship_navigate_explore_move() {
    JSON.test(
        "ship_navigate_explore_move",
        vec![TestAction::not_undoable(
            1,
            move_action(vec![1], Position::from_offset("F2")),
        )],
    );
}

#[test]
fn test_ship_navigate_explore_not_move() {
    JSON.test(
        "ship_navigate_explore_not_move",
        vec![TestAction::not_undoable(
            1,
            move_action(vec![1], Position::from_offset("F2")),
        )],
    );
}

// Rulebook pp.14,25: exhaustion suppresses collection/founding, not terrain movement.
#[test]
fn exhausted_terrain_retains_turn_movement_restrictions() {
    use server::action::{execute_action, try_execute_action};
    use server::game::GameState;
    use server::map::Terrain;
    use server::movement::MovementAction;
    use server::unit::UnitType;

    for terrain in [Terrain::Forest, Terrain::Mountain, Terrain::Fertile] {
        for exhausted in [false, true] {
            for unit_type in [UnitType::Infantry, UnitType::Settler] {
                let mut game = JSON.load_game("movement");
                game.state = GameState::Playing;
                game.actions_left = 3;
                game.player_mut(0)
                    .advances
                    .insert(server::advance::Advance::Tactics);
                let start = Position::new(0, 0);
                let entered = Position::new(1, 0);
                let peaceful = Position::new(2, 0);
                let enemy = Position::new(1, 1);
                game.map.tiles.clear();
                for pos in [start, entered, peaceful, enemy] {
                    game.map.tiles.insert(pos, Terrain::Barren);
                }
                game.map.tiles.insert(
                    entered,
                    if exhausted {
                        Terrain::Exhausted(Box::new(terrain.clone()))
                    } else {
                        terrain.clone()
                    },
                );
                for p in &mut game.players {
                    p.cities.clear();
                    p.units.clear();
                }
                game.player_mut(0).units.push(server::unit::Unit::new(
                    0,
                    start,
                    unit_type.clone(),
                    0,
                ));
                let id = game.player(0).units[0].id;
                game.player_mut(0).units.push(server::unit::Unit::new(
                    0,
                    entered,
                    UnitType::Infantry,
                    1,
                ));
                let fresh = game.player(0).units[1].id;
                game.player_mut(1).units.push(server::unit::Unit::new(
                    1,
                    enemy,
                    UnitType::Infantry,
                    0,
                ));
                // Starting on the terrain is not entering it.
                assert!(
                    possible_move_routes(game.player(0), &game, &[fresh], entered, None)
                        .unwrap()
                        .iter()
                        .any(|r| r.destination == peaceful)
                );
                game = execute_action(game, move_action(vec![id], entered), 0);
                // Same Move action forbids moving the same units, independently of terrain.
                assert!(
                    try_execute_action(game.clone(), move_action(vec![id], peaceful), 0).is_err()
                );
                game = execute_action(game, Action::Movement(MovementAction::Stop), 0);
                let routes = |ids: &[u32]| {
                    possible_move_routes(game.player(0), &game, ids, entered, None)
                        .unwrap_or_default()
                };
                assert_eq!(
                    routes(&[id]).iter().any(|r| r.destination == peaceful),
                    terrain != Terrain::Mountain,
                    "{terrain:?}, exhausted={exhausted}, {unit_type:?}"
                );
                if unit_type == UnitType::Infantry {
                    assert_eq!(
                        routes(&[id]).iter().any(|r| r.destination == enemy),
                        terrain == Terrain::Fertile
                    );
                }
                assert!(routes(&[fresh]).iter().any(|r| r.destination == peaceful));
                assert_eq!(
                    routes(&[id, fresh])
                        .iter()
                        .any(|r| r.destination == peaceful),
                    terrain != Terrain::Mountain
                );
                if terrain == Terrain::Mountain {
                    assert!(
                        try_execute_action(game.clone(), move_action(vec![id], peaceful), 0)
                            .is_err()
                    );
                } else {
                    assert!(
                        try_execute_action(game.clone(), move_action(vec![id], peaceful), 0)
                            .is_ok()
                    );
                }
                if unit_type == UnitType::Infantry && terrain != Terrain::Fertile {
                    assert!(
                        try_execute_action(game.clone(), move_action(vec![id], enemy), 0).is_err()
                    );
                }
                server::player::end_turn(&mut game, 0);
                assert!(game.player(0).get_unit(id).movement_restrictions.is_empty());
                assert!(
                    possible_move_routes(game.player(0), &game, &[id], entered, None)
                        .unwrap()
                        .iter()
                        .any(|r| r.destination == peaceful)
                );
            }
        }
    }
}

#[test]
fn exhaustion_still_blocks_collection_and_founding() {
    use server::action::try_execute_action;
    use server::collect::possible_resource_collections;
    use server::events::check_event_origin;
    use server::game::GameState;
    use server::map::Terrain;
    use server::player::CostTrigger;
    use server::playing_actions::PlayingAction;

    for terrain in [Terrain::Forest, Terrain::Mountain, Terrain::Fertile] {
        let mut game = JSON.load_game("movement");
        game.state = GameState::Playing;
        let city = Position::from_offset("A1");
        let site = Position::from_offset("B2");
        for pos in [city, site] {
            game.map
                .tiles
                .insert(pos, Terrain::Exhausted(Box::new(terrain.clone())));
        }
        let options = possible_resource_collections(
            &game,
            city,
            0,
            &check_event_origin(),
            CostTrigger::NoModifiers,
        );
        assert!(!options.choices.contains_key(&city));
        assert!(!options.choices.contains_key(&site));
        assert!(
            try_execute_action(
                game,
                Action::Playing(PlayingAction::FoundCity { settler: 4 }),
                0
            )
            .is_err()
        );
    }
}
