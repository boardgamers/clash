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

#[test]
fn fleet_boarding_assignments_are_atomic_and_share_a_movement_group() {
    use server::{
        action::try_execute_action,
        advance::Advance,
        game::GameState,
        map::Terrain,
        unit::{Unit, UnitType},
    };
    for pirates in [false, true] {
        for split in [false, true] {
            for count in [2, 4] {
                let mut game = JSON.load_game("movement");
                game.state = GameState::Playing;
                game.actions_left = 3;
                let start = Position::from_offset("C8");
                let sea = Position::from_offset("D8");
                game.map.tiles.clear();
                game.map.tiles.insert(start, Terrain::Barren);
                game.map.tiles.insert(sea, Terrain::Water);
                game.map
                    .tiles
                    .insert(Position::from_offset("E8"), Terrain::Water);
                for p in &mut game.players {
                    p.units.clear();
                    p.cities.clear();
                }
                game.player_mut(0).advances.insert(Advance::Tactics);
                for id in 0..count {
                    game.player_mut(0).units.push(Unit::new(
                        0,
                        start,
                        if id < 2 {
                            UnitType::Infantry
                        } else {
                            UnitType::Elephant
                        },
                        id,
                    ));
                }
                game.player_mut(0).next_unit_id = 22;
                if pirates {
                    game.player_mut(0).civilization = game.cache.get_civilization("Carthage");
                    game.player_mut(0).advances.insert(Advance::Navigation);
                    game.players.push(server::player::Player::new(
                        game.cache.get_civilization("Pirates"),
                        2,
                    ));
                    game.player_mut(0)
                        .special_advances
                        .insert(serde_json::from_value(serde_json::json!("PirateAllies")).unwrap());
                    let owner = game
                        .players
                        .iter()
                        .position(|p| p.civilization.name == "Pirates")
                        .unwrap();
                    for id in [2, 3] {
                        game.player_mut(owner).units.push(Unit::new(
                            owner,
                            sea,
                            UnitType::Ship,
                            id,
                        ));
                    }
                } else {
                    for id in [10, 11] {
                        game.player_mut(0)
                            .units
                            .push(Unit::new(0, sea, UnitType::Ship, id));
                    }
                }
                let mut m = MoveUnits::new((0..count).collect(), sea, None, ResourcePile::empty());
                let first = if split { vec![0] } else { vec![0, 1] };
                let second: Vec<_> = (0..count).filter(|id| !first.contains(id)).collect();
                m.boarding.push(MoveUnits::new(
                    first.clone(),
                    sea,
                    Some(10),
                    ResourcePile::empty(),
                ));
                if !second.is_empty() {
                    m.boarding.push(MoveUnits::new(
                        second.clone(),
                        sea,
                        Some(11),
                        ResourcePile::empty(),
                    ));
                }
                if pirates {
                    for group in &mut m.boarding {
                        group.embark_pirate = group.embark_carrier_id.take().map(|id| id - 8);
                    }
                }
                let before = serde_json::to_value(game.clone().data()).unwrap();
                let result = try_execute_action(game.clone(), Action::Movement(Move(m.clone())), 0);
                if count == 4 && split {
                    assert!(result.is_err());
                    assert_eq!(serde_json::to_value(game.clone().data()).unwrap(), before);
                    continue;
                }
                game = result.unwrap();
                assert_eq!(game.actions_left, 2);
                for id in first {
                    assert_eq!(
                        game.player(0).get_unit(id).carrier_id,
                        Some(if pirates { 22 } else { 10 })
                    );
                }
                for id in second {
                    assert_eq!(
                        game.player(0).get_unit(id).carrier_id,
                        Some(if pirates { 23 } else { 11 })
                    );
                }
                if let GameState::Movement(state) = &game.state {
                    assert_eq!(state.movement_actions_left, 2);
                } else {
                    panic!("expected movement");
                }
                // A repeated assignment cannot transfer passengers at sea or charge again.
                let before = serde_json::to_value(game.clone().data()).unwrap();
                assert!(try_execute_action(game.clone(), Action::Movement(Move(m)), 0).is_err());
                assert_eq!(serde_json::to_value(game.clone().data()).unwrap(), before);
            }
        }
    }
}

#[test]
fn legacy_move_payload_omits_optional_fleet_assignment() {
    let legacy = serde_json::json!({"units":[1],"destination":"D8","embark_carrier_id":10});
    let action: MoveUnits = serde_json::from_value(legacy.clone()).unwrap();
    assert!(action.boarding.is_empty());
    assert_eq!(serde_json::to_value(action).unwrap(), legacy);
}

fn setup_disembark_game(
    dest_terrain: server::map::Terrain,
    carried: &[server::unit::UnitType],
    friendly_on_dest: &[server::unit::UnitType],
    enemy_on_dest: &[server::unit::UnitType],
) -> (Game, Position, Position, Vec<u32>) {
    use server::{
        advance::Advance,
        game::GameState,
        map::Terrain,
        unit::{Unit, UnitType},
    };

    let mut game = JSON.load_game("movement");
    game.state = GameState::Playing;
    game.actions_left = 3;
    let sea = Position::from_offset("D8");
    let land = Position::from_offset("C8");
    game.map.tiles.clear();
    game.map.tiles.insert(sea, Terrain::Water);
    game.map.tiles.insert(land, dest_terrain);
    for p in &mut game.players {
        p.units.clear();
        p.cities.clear();
    }
    game.player_mut(0).advances.insert(Advance::Tactics);

    let ships_needed = carried.len().div_ceil(2).max(1) as u32;
    for s in 0..ships_needed {
        game.player_mut(0)
            .units
            .push(Unit::new(0, sea, UnitType::Ship, 100 + s));
    }

    let mut carried_ids = Vec::with_capacity(carried.len());
    for (idx, &unit_type) in carried.iter().enumerate() {
        let id = idx as u32;
        let mut unit = Unit::new(0, sea, unit_type, id);
        unit.carrier_id = Some(100 + (idx / 2) as u32);
        game.player_mut(0).units.push(unit);
        carried_ids.push(id);
    }

    for (idx, &unit_type) in friendly_on_dest.iter().enumerate() {
        let id = 200 + idx as u32;
        game.player_mut(0)
            .units
            .push(Unit::new(0, land, unit_type, id));
    }

    for (idx, &unit_type) in enemy_on_dest.iter().enumerate() {
        let id = 300 + idx as u32;
        game.player_mut(1)
            .units
            .push(Unit::new(1, land, unit_type, id));
    }

    game.player_mut(0).next_unit_id = 400;
    game.player_mut(1).next_unit_id = 400;
    (game, sea, land, carried_ids)
}

#[test]
fn disembark_enforces_stack_limit_on_empty_and_enemy_land() {
    use server::{action::try_execute_action, leader::Leader, map::Terrain, unit::UnitType};

    let five_army_plus_settler = [
        UnitType::Infantry,
        UnitType::Infantry,
        UnitType::Infantry,
        UnitType::Infantry,
        UnitType::Leader(Leader::Alexander),
        UnitType::Settler,
    ];

    for enemy in [&[][..], &[UnitType::Infantry][..]] {
        let (game, sea, land, ids) =
            setup_disembark_game(Terrain::Barren, &five_army_plus_settler, &[], enemy);
        let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
        assert!(
            !routes.iter().any(|r| r.destination == land),
            "5 army units + settler must not be able to disembark onto {enemy:?}"
        );
        assert!(
            try_execute_action(game, move_action(ids, land), 0).is_err(),
            "disembarking 5 army units + settler must be rejected"
        );
    }

    // 4 Infantry + 1 Settler succeeds
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Settler,
        ],
        &[],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(routes.iter().any(|r| r.destination == land));
    let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
    for id in ids {
        assert_eq!(after.player(0).get_unit(id).position, land);
        assert_eq!(after.player(0).get_unit(id).carrier_id, None);
    }

    // 3 Infantry + non-Xerxes Leader + 1 Settler succeeds
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Leader(Leader::Alexander),
            UnitType::Settler,
        ],
        &[],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(routes.iter().any(|r| r.destination == land));
    let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
    for id in ids {
        assert_eq!(after.player(0).get_unit(id).position, land);
        assert_eq!(after.player(0).get_unit(id).carrier_id, None);
    }
}

#[test]
fn disembark_enforces_stack_limit_with_existing_friendly_units_and_allows_settlers() {
    use server::{action::try_execute_action, map::Terrain, unit::UnitType};

    // 2 Infantry disembarking onto 3 existing army units (5 > 4) is rejected
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[UnitType::Infantry, UnitType::Infantry],
        &[UnitType::Infantry, UnitType::Infantry, UnitType::Infantry],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(!routes.iter().any(|r| r.destination == land));
    assert!(try_execute_action(game, move_action(ids, land), 0).is_err());

    // 1 Infantry disembarking onto 3 existing army units (4 <= 4) succeeds
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[UnitType::Infantry],
        &[UnitType::Infantry, UnitType::Infantry, UnitType::Infantry],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(routes.iter().any(|r| r.destination == land));
    let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
    assert_eq!(after.player(0).get_unit(ids[0]).position, land);

    // 1 Settler disembarking onto 4 or legacy 5 existing army units succeeds
    for existing_count in [4, 5] {
        let existing = vec![UnitType::Infantry; existing_count];
        let (game, sea, land, ids) =
            setup_disembark_game(Terrain::Barren, &[UnitType::Settler], &existing, &[]);
        let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
        assert!(
            routes.iter().any(|r| r.destination == land),
            "settler should disembark onto {existing_count} existing army units"
        );
        let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
        assert_eq!(after.player(0).get_unit(ids[0]).position, land);
    }
}

#[test]
fn disembark_respects_xerxes_stack_limit() {
    use server::{action::try_execute_action, leader::Leader, map::Terrain, unit::UnitType};

    // Xerxes + 4 Infantry (5) succeeds
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[
            UnitType::Leader(Leader::Xerxes),
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
        ],
        &[],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(routes.iter().any(|r| r.destination == land));
    let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
    for id in ids {
        assert_eq!(after.player(0).get_unit(id).position, land);
    }

    // Xerxes + 5 Infantry (6) is rejected
    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[
            UnitType::Leader(Leader::Xerxes),
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
        ],
        &[],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(!routes.iter().any(|r| r.destination == land));
    assert!(try_execute_action(game, move_action(ids, land), 0).is_err());
}

#[test]
fn disembark_enforces_stack_limit_on_unexplored_tile() {
    use server::{action::try_execute_action, map::Terrain, unit::UnitType};

    let (game, sea, land, ids) = setup_disembark_game(
        Terrain::Unexplored,
        &[
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
        ],
        &[],
        &[],
    );
    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(!routes.iter().any(|r| r.destination == land));
    assert!(try_execute_action(game, move_action(ids, land), 0).is_err());
}

#[test]
fn disembark_allows_legacy_overstack_during_replay() {
    use server::{
        action::try_execute_action, game::GameContext, leader::Leader, map::Terrain, unit::UnitType,
    };

    let (mut game, sea, land, ids) = setup_disembark_game(
        Terrain::Barren,
        &[
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Infantry,
            UnitType::Leader(Leader::Alexander),
            UnitType::Settler,
        ],
        &[],
        &[],
    );
    game.context = GameContext::Replay;

    let routes = possible_move_routes(game.player(0), &game, &ids, sea, None).unwrap();
    assert!(routes.iter().any(|r| r.destination == land));
    let after = try_execute_action(game, move_action(ids.clone(), land), 0).unwrap();
    for id in ids {
        assert_eq!(after.player(0).get_unit(id).position, land);
        assert_eq!(after.player(0).get_unit(id).carrier_id, None);
    }
}
