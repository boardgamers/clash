use crate::common::{JsonTest, TestAction, custom_action, move_action, payment_response};
use server::action::Action;
use server::content::custom_actions::CustomActionType;
use server::content::persistent_events::EventResponse;
use server::movement::{MoveUnits, MovementAction};
use server::playing_actions::PlayingAction::EndTurn;
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

const JSON: JsonTest = JsonTest::child("civilizations", "vikings");

#[test]
fn ship_construction() {
    JSON.test(
        "ship_construction",
        vec![
            TestAction::undoable(0, move_action(vec![3], Position::from_offset("D2"))).skip_json(),
            // embark in same current move
            TestAction::undoable(
                0,
                Action::Movement(MovementAction::Move(MoveUnits::new(
                    vec![4, 5],
                    Position::from_offset("D2"),
                    Some(3),
                    ResourcePile::empty(),
                ))),
            )
            .skip_json(),
            TestAction::undoable(0, move_action(vec![3], Position::from_offset("C2"))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![3]))),
        ],
    );
}

#[test]
fn longships() {
    JSON.test(
        "longships",
        vec![
            TestAction::not_undoable(0, move_action(vec![3, 4, 5], Position::from_offset("C3")))
                .skip_json(),
            TestAction::not_undoable(0, move_action(vec![4, 5], Position::from_offset("C4"))),
        ],
    )
}

#[test]
fn raids() {
    JSON.test(
        "raids",
        vec![
            TestAction::not_undoable(0, Action::Playing(EndTurn)).skip_json(),
            TestAction::not_undoable(1, Action::Playing(EndTurn)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::food(1))),
        ],
    );
}

#[test]
fn danegeld() {
    JSON.test(
        "danegeld",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::Danegeld)).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::food(4))),
            ),
        ],
    );
}

#[test]
fn explorer() {
    JSON.test(
        "explorer",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::LegendaryExplorer)).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))),
        ],
    );
}

#[test]
fn new_colonies() {
    JSON.test(
        "new_colonies",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::NewColonies)).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
                    "D1",
                )])),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0]))),
        ],
    );
}

#[test]
fn prey() {
    JSON.test(
        "prey",
        vec![TestAction::undoable(
            0,
            move_action(vec![0], Position::from_offset("D1")),
        )],
    )
}

#[test]
fn converting_ship_carries_army_passengers_in_one_group_move() {
    use server::unit::UnitType;
    let mut game = JSON.load_game("ship_construction");
    let sea = Position::from_offset("D2");
    let land = Position::from_offset("C2");
    game.players[0].units.truncate(3);
    game.players[0].units[0].unit_type = UnitType::Ship;
    for u in &mut game.players[0].units {
        u.position = sea;
        if u.id != 3 {
            u.unit_type = UnitType::Infantry;
            u.carrier_id = Some(3);
        }
    }
    game = server::game_api::execute(game, move_action(vec![3], land), 0);
    game = server::game_api::execute(
        game,
        Action::Response(EventResponse::SelectUnits(vec![])),
        0,
    );
    assert!(
        game.players[0]
            .units
            .iter()
            .all(|u| u.position == land && u.carrier_id.is_none())
    );
    assert!(
        game.players[0]
            .units
            .iter()
            .all(|u| u.unit_type == UnitType::Infantry)
    );
    if let server::game::GameState::Movement(m) = &game.state {
        assert_eq!(m.movement_actions_left, 2);
    } else {
        panic!("expected movement");
    }
}

// Synthetic legal cargo, not a historical save. Exercise both the live callback
// and a suspended conversion restored across the ordinary save boundary.
#[test]
fn converting_ship_unloads_leader_cargo() {
    use serde_json::json;
    use server::{
        cache::Cache,
        game::{Game, GameContext},
        game_data::GameData,
        leader::Leader,
        unit::UnitType,
    };
    fn load(value: serde_json::Value) -> Game {
        let data: GameData = serde_json::from_value(value).unwrap();
        let cache = Cache::new(&data.options);
        Game::from_data(data, cache, GameContext::Play)
    }
    for mixed in [false, true] {
        for settler in [false, true] {
            for resume in [false, true] {
                let mut value: serde_json::Value = serde_json::from_str(include_str!(
                    "test_games/civilizations/vikings/ship_construction.json"
                ))
                .unwrap();
                value["players"].as_array_mut().unwrap().swap(0, 1);
                value["players"][0]["id"] = json!(0);
                value["players"][1]["id"] = json!(1);
                value["current_player_index"] = json!(1);
                value["starting_player_index"] = json!(1);
                value["log"][0]["rounds"][0]["turns"][0]["turn_type"]["Player"] = json!(1);
                let p = &mut value["players"][1];
                p["recruited_leaders"] = json!(["Ragnar"]);
                p["units"] = json!([
                    {"id":3,"position":"D2","unit_type":"Ship"},
                    {"id":4,"position":"D2","unit_type":{"Leader":"Ragnar"},"carrier_id":3}
                ]);
                if mixed {
                    p["units"].as_array_mut().unwrap().push(
                        json!({"id":5,"position":"D2","unit_type":"Infantry","carrier_id":3}),
                    );
                }
                let mut game = server::game_api::execute(
                    load(value),
                    move_action(vec![3], Position::from_offset("C2")),
                    1,
                );
                assert_eq!(game.player(1).get_unit(4).carrier_id, Some(3));
                if resume {
                    game = load(serde_json::to_value(game.cloned_data()).unwrap());
                }
                game = server::game_api::execute(
                    game,
                    Action::Response(EventResponse::SelectUnits(if settler {
                        vec![3]
                    } else {
                        vec![]
                    })),
                    1,
                );
                let p = game.player(1);
                assert_eq!(p.units.len(), if mixed { 3 } else { 2 });
                assert_eq!(
                    p.get_unit(3).unit_type,
                    if settler {
                        UnitType::Settler
                    } else {
                        UnitType::Infantry
                    }
                );
                assert_eq!(p.get_unit(4).unit_type, UnitType::Leader(Leader::Ragnar));
                if mixed {
                    assert_eq!(p.get_unit(5).unit_type, UnitType::Infantry);
                }
                assert!(
                    p.units
                        .iter()
                        .all(|u| u.position == Position::from_offset("C2")
                            && u.carrier_id.is_none()
                            && u.player_index == 1)
                );
                if let server::game::GameState::Movement(m) = &game.state {
                    assert_eq!(m.movement_actions_left, 2);
                    assert_eq!(m.moved_units, vec![3]);
                } else {
                    panic!("expected movement");
                }
                let log = serde_json::to_value(game.cloned_data()).unwrap()["log"].to_string();
                let expected = if mixed {
                    "Unloaded 1 infantry and Ragnar Lodbrok from ships"
                } else {
                    "Unloaded Ragnar Lodbrok from ships"
                };
                assert_eq!(log.matches(expected).count(), 1);
            }
        }
    }
}
