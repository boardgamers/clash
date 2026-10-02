use crate::common::{JsonTest, TestAction, move_action};
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::recruit;
use server::resource_pile::ResourcePile;
use server::unit::Units;

mod common;

const JSON: JsonTest = JsonTest::child("objective_cards", "instant");

fn naval_assault_game() -> server::game::Game {
    use server::city::City;
    use server::map::Terrain;
    use server::unit::{Unit, UnitType};
    let mut game = JsonTest::new("tactics_cards").load_game("heavy_resistance");
    let sea = Position::from_offset("C2");
    game.map.tiles.insert(sea, Terrain::Water);
    game.players[0].cities.retain(|city| city.position != sea);
    game.players[0]
        .units
        .retain(|unit| [0, 2].contains(&unit.id));
    for unit in &mut game.players[0].units {
        unit.carrier_id = Some(9);
    }
    game.players[0]
        .units
        .push(Unit::new(0, sea, UnitType::Ship, 9));
    game.players[0].next_unit_id = 10;
    game.players[0].objective_cards = vec![10, 20];
    game.players[0].action_cards.clear();
    game.players[1].action_cards.clear();
    game.players[1].objective_cards.clear();
    game.players[1]
        .advances
        .remove(server::advance::Advance::Fanaticism);
    game.players[1].units.truncate(1);
    for pos in ["B1", "D2"] {
        game.players[1]
            .cities
            .push(City::new(1, Position::from_offset(pos)));
    }
    game.dice_roll_outcomes = vec![0, 11, 11];
    game.clone()
}

#[test]
fn naval_assault_survives_the_defenders_settler_choice_and_save_reload() {
    use server::content::persistent_events::PersistentEventRequest;
    use server::game_api;
    let game = game_api::execute(
        naval_assault_game(),
        move_action(vec![0, 2], Position::from_offset("C1")),
        0,
    );
    assert_eq!(
        game.active_player(),
        1,
        "defender places their replacement settler first"
    );
    for seat in [Some(0), Some(1), None] {
        let visible = game_api::strip_secret(game.clone(), seat);
        let data = serde_json::to_value(visible.cloned_data()).unwrap();
        if seat == Some(0) {
            assert_eq!(
                data["players"][0]["objective_opportunities"],
                serde_json::json!(["Naval Assault"])
            );
        } else {
            assert!(
                data["players"][0].get("objective_opportunities").is_none(),
                "queued objectives stay private"
            );
        }
    }
    let game = game.clone(); // mirrors the server saving and loading between players
    let mut game = game_api::execute(
        game,
        Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
            "B1",
        )])),
        1,
    );
    assert_eq!(game.active_player(), 0);
    let handler = game
        .current_event_handler_mut()
        .expect("Naval Assault claim must be offered after the settler placement");
    let PersistentEventRequest::SelectHandCards(request) = &handler.request else {
        panic!("expected an objective card choice")
    };
    assert_eq!(
        request.choices,
        vec![HandCard::ObjectiveCard(10), HandCard::ObjectiveCard(20)]
    );
    assert!(request.description.contains("Naval Assault"));
    let game = game_api::execute(
        game,
        Action::Response(EventResponse::SelectHandCards(vec![
            HandCard::ObjectiveCard(20),
        ])),
        0,
    );
    assert_eq!(game.player(0).completed_objectives.len(), 1);
    assert_eq!(game.player(0).completed_objectives[0].name, "Naval Assault");
    assert_eq!(
        game.player(0).objective_cards,
        vec![10],
        "one capture only completes one copy"
    );
    assert!(game.events.is_empty());
}

#[test]
fn repelling_a_landing_does_not_complete_naval_assault_for_the_defender() {
    use server::game_api;
    let mut game = naval_assault_game();
    game.players[0].objective_cards.clear();
    game.players[1].objective_cards = vec![10, 20];
    game.dice_roll_outcomes = vec![11, 0];
    let game = game_api::execute(
        game.clone(),
        move_action(vec![0], Position::from_offset("C1")),
        0,
    );
    assert!(
        game.player(1)
            .try_get_city(Position::from_offset("C1"))
            .is_some()
    );
    assert!(
        game.events.is_empty(),
        "defender did not capture a city by disembarking"
    );
    assert!(game.player(1).completed_objectives.is_empty());
}

#[test]
fn test_draft() {
    let r = Action::Playing(PlayingAction::Recruit(recruit::Recruit::new(
        &Units::new(0, 1, 0, 0, 0, None),
        Position::from_offset("A1"),
        ResourcePile::mood_tokens(1),
    )));
    JSON.test(
        "draft",
        vec![
            TestAction::undoable(0, r.clone()).skip_json(),
            TestAction::undoable(0, r).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(1),
                ])),
            ),
        ],
    )
}

#[test]
fn test_conqueror() {
    JSON.test(
        "conqueror",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(2),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(5),
                ])),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(6),
                ])),
            ),
        ],
    );
}

#[test]
fn test_defiance() {
    JSON.test(
        "defiance",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectHandCards(vec![])))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(7),
                ])),
            ),
        ],
    );
}

#[test]
fn test_warmonger() {
    JSON.test(
        "warmonger",
        vec![
            TestAction::not_undoable(0, move_action(vec![0, 1], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(0, move_action(vec![2, 3], Position::from_offset("B1")))
                .skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(3),
                ])),
            ),
        ],
    );
}

#[test]
fn test_scavenger() {
    JSON.test(
        "scavenger",
        vec![
            TestAction::not_undoable(0, move_action(vec![0, 1], Position::from_offset("E7")))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(21),
                ])),
            ),
        ],
    );
}
