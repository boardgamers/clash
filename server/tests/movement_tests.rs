use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::game::Game;
use server::movement::MoveUnits;
use server::movement::MovementAction::Move;
use server::movement::move_units_destinations;
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
            Action::Movement(Move(MoveUnits {
                units: vec![3, 4],
                destination: Position::from_offset("C3"),
                embark_carrier_id: Some(8),
                payment: ResourcePile::empty(),
            })),
        )],
    );
}

#[test]
fn test_ship_embark_continue() {
    JSON.test(
        "ship_embark_continue",
        vec![TestAction::undoable(
            0,
            Action::Movement(Move(MoveUnits {
                units: vec![5, 6],
                destination: Position::from_offset("C3"),
                embark_carrier_id: Some(9),
                payment: ResourcePile::empty(),
            })),
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
                .without_json_comparison(),
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
        vec![TestAction::not_undoable(
            1,
            move_action(vec![1], Position::from_offset("C5")),
        )],
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
    set_unit_position(1, 1, from, game);
    let result = move_units_destinations(game.player(1), game, &[1], from, None)
        .is_ok_and(|d| d.iter().any(|route| route.destination == to));
    assert!(
        result,
        "expected to be able to move from {} to {}",
        from, to,
    );
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
