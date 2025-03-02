use crate::common::{load_game, move_action, test_action};
use server::action::Action;
use server::game::Game;
use server::movement::move_units_destinations;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::MoveUnits;
use server::unit::MovementAction::Move;

mod common;

#[test]
fn test_movement() {
    test_action(
        "movement",
        move_action(vec![4], Position::from_offset("B3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_explore_choose() {
    test_action(
        "explore_choose",
        move_action(vec![0], Position::from_offset("C7")),
        1,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_no_walk_on_water() {
    test_action(
        "explore_auto_no_walk_on_water",
        move_action(vec![0], Position::from_offset("B2")),
        0,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_adjacent_water() {
    test_action(
        "explore_auto_adjacent_water",
        move_action(vec![0], Position::from_offset("C7")),
        0,
        false,
        false,
    );
}

#[test]
fn test_explore_auto_water_outside() {
    test_action(
        "explore_auto_water_outside",
        move_action(vec![1], Position::from_offset("F5")),
        1,
        false,
        false,
    );
}

#[test]
fn test_explore_resolution() {
    test_action(
        "explore_resolution",
        Action::ExploreResolution(3),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_transport() {
    // undo capture empty city is broken
    test_action(
        "ship_transport",
        move_action(vec![7], Position::from_offset("D2")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_transport_same_sea() {
    // undo capture empty city is broken
    test_action(
        "ship_transport_same_sea",
        move_action(vec![7], Position::from_offset("C3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_embark() {
    test_action(
        "ship_embark",
        Action::Movement(Move(MoveUnits {
            units: vec![3, 4],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(8),
            payment: ResourcePile::empty(),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_embark_continue() {
    test_action(
        "ship_embark_continue",
        Action::Movement(Move(MoveUnits {
            units: vec![5, 6],
            destination: Position::from_offset("C3"),
            embark_carrier_id: Some(9),
            payment: ResourcePile::empty(),
        })),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_disembark() {
    // undo capture empty city is broken
    test_action(
        "ship_disembark",
        move_action(vec![1, 2], Position::from_offset("B3")),
        0,
        true,
        false,
    );
}

#[test]
fn test_ship_disembark_capture_empty_city() {
    test_action(
        "ship_disembark_capture_empty_city",
        move_action(vec![1, 2], Position::from_offset("B2")),
        0,
        false,
        false,
    );
}

#[test]
fn test_ship_explore() {
    test_action(
        "ship_explore",
        move_action(vec![1], Position::from_offset("C5")),
        1,
        false,
        false,
    );
}

#[test]
fn test_ship_explore_teleport() {
    test_action(
        "ship_explore_teleport",
        move_action(vec![1], Position::from_offset("C4")),
        1,
        false,
        false,
    );
}

#[test]
fn test_ship_explore_move_not_possible() {
    test_action(
        "ship_explore_move_not_possible",
        Action::ExploreResolution(3),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_navigate() {
    test_action(
        "ship_navigate",
        move_action(vec![1], Position::from_offset("A7")),
        1,
        true,
        false,
    );
}

#[test]
fn test_ship_navigate_coordinates() {
    let mut game = load_game("ship_navigation_unit_test");

    let pairs = [
        ("B3", "B5"),
        ("B5", "A7"),
        ("A7", "F7"),
        ("G7", "G3"),
        ("G3", "B3"),
    ];

    for pair in pairs {
        let from = Position::from_offset(pair.0);
        let to = Position::from_offset(pair.1);
        assert_navigate(&mut game, from, to);
        assert_navigate(&mut game, to, from);
    }
}

fn assert_navigate(game: &mut Game, from: Position, to: Position) {
    game.players[1].get_unit_mut(1).unwrap().position = from;
    let result = move_units_destinations(game.get_player(1), game, &[1], from, None)
        .is_ok_and(|d| d.iter().any(|route| route.destination == to));
    assert!(
        result,
        "expected to be able to move from {} to {}",
        from, to,
    );
}

#[test]
fn test_ship_navigate_explore_move() {
    test_action(
        "ship_navigate_explore_move",
        move_action(vec![1], Position::from_offset("F2")),
        1,
        false,
        false,
    );
}

#[test]
fn test_ship_navigate_explore_not_move() {
    test_action(
        "ship_navigate_explore_not_move",
        move_action(vec![1], Position::from_offset("F2")),
        1,
        false,
        false,
    );
}
