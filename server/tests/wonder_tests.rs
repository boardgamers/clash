use crate::common::{
    JsonTest, TestAction, advance_action, custom_action, move_action, payment_response,
};
use server::action::{Action, execute_without_undo};
use server::advance::Advance;
use server::card::HandCard;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::content::persistent_events::EventResponse;
use server::playing_actions::PlayingAction;
use server::playing_actions::PlayingAction::WonderCard;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::wonder::Wonder;

mod common;

const JSON: JsonTest = JsonTest::new("wonders");

#[test]
fn test_colosseum() {
    JSON.test(
        "colosseum",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Colosseum))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(3, 4, 5, 0, 0, 0, 5)))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Playing(PlayingAction::Custom(CustomAction::new(
                    CustomActionType::Sports,
                    Some(Position::from_offset("C2")),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                // mood payment is only possible because of colosseum
                payment_response(ResourcePile::mood_tokens(1)),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::culture_tokens(1))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0]))),
        ],
    );
}

#[test]
fn test_pyramids() {
    JSON.test(
        "pyramids",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Pyramids))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(2, 3, 6, 0, 1, 0, 5)))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(32),
                ])),
            ),
        ],
    );
}

#[test]
fn test_library() {
    JSON.test(
        "library",
        vec![
            TestAction::undoable(0, custom_action(CustomActionType::GreatLibrary)).skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectAdvance(Advance::Engineering)),
            )
            .skip_json(),
            // can use effect to build a wonder - but don't draw a wonder card (one time ability)
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::Pyramids))),
        ],
    );
}

#[test]
fn test_lighthouse() {
    let game = JSON.load_game("lighthouse");
    let game = server::game_api::execute(
        game,
        Action::Playing(WonderCard(Wonder::GreatLighthouse)),
        0,
    );
    let game = server::game_api::execute(
        game,
        payment_response(ResourcePile::new(3, 5, 4, 0, 0, 0, 5)),
        0,
    );
    let city = game
        .player(0)
        .cities
        .iter()
        .find(|c| c.pieces.wonders.contains(&Wonder::GreatLighthouse))
        .unwrap();
    let activations = city.activations;
    let position = city.position;
    let actions = game.actions_left;
    let ships = game
        .player(0)
        .units
        .iter()
        .filter(|u| u.unit_type.is_ship())
        .count();
    let game = server::game_api::execute(
        game,
        Action::Response(EventResponse::SelectPositions(vec![Position::from_offset(
            "C3",
        )])),
        0,
    );
    assert_eq!(game.player(0).get_city(position).activations, activations);
    assert_eq!(game.actions_left, actions);
    assert_eq!(
        game.player(0)
            .units
            .iter()
            .filter(|u| u.unit_type.is_ship())
            .count(),
        ships + 1
    );
    assert!(!game.player(0).special_actions.contains_key(
        &server::content::custom_actions::SpecialAction::Custom(CustomActionType::GreatLighthouse)
    ));
}

#[test]
fn test_great_gardens() {
    JSON.test(
        "great_gardens",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatGardens))).skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::new(5, 5, 2, 0, 0, 0, 5)))
                .skip_json(),
            TestAction::not_undoable(0, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::undoable(1, move_action(vec![0, 1], Position::from_offset("B1")))
                .with_post_assert(|mut game| {
                    let result = execute_without_undo(
                        &mut game,
                        move_action(vec![0, 1], Position::from_offset("A1")),
                        1,
                    );
                    assert_eq!(
                        result.err(),
                        Some("fertile movement attack great gardens restriction".to_string())
                    );
                }),
        ],
    );
}

#[test]
fn test_great_mausoleum() {
    JSON.test(
        "great_mausoleum",
        vec![
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(123))).skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))).skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))).skip_json(),
            TestAction::undoable(0, advance_action(Advance::Storage, ResourcePile::ideas(2)))
                .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

#[test]
fn test_great_statue() {
    JSON.test(
        "great_statue",
        vec![
            TestAction::undoable(0, Action::Playing(WonderCard(Wonder::GreatStatue))).skip_json(),
            TestAction::not_undoable(0, payment_response(ResourcePile::new(3, 4, 5, 0, 0, 0, 5)))
                .skip_json(),
            TestAction::undoable(0, custom_action(CustomActionType::GreatStatue)),
        ],
    );
}

#[test]
fn test_great_wall() {
    JSON.test(
        "great_wall",
        vec![
            TestAction::not_undoable(1, move_action(vec![0, 1], Position::from_offset("A1")))
                .skip_json(),
            TestAction::not_undoable(1, Action::Playing(PlayingAction::EndTurn)).skip_json(),
            TestAction::not_undoable(0, advance_action(Advance::Storage, ResourcePile::ideas(2)))
                .skip_json(),
            TestAction::undoable(0, Action::Playing(PlayingAction::ActionCard(5))),
        ],
    );
}

#[test]
fn great_mausoleum_previews_visible_discards() {
    let game = JSON.load_game("great_mausoleum");
    let game = server::game_api::execute(game, Action::Playing(PlayingAction::ActionCard(123)), 0);
    let game = server::game_api::execute(game, Action::Response(EventResponse::Bool(true)), 0);
    let displayed = server::web_view::view(&game, Some(0));
    let preview = &displayed["choiceDecision"]["preview"];
    assert!(preview["name"].is_string());
    assert!(preview["rules"].as_array().unwrap().len() >= 2);
    let game = server::game_api::execute(game, Action::Response(EventResponse::Bool(false)), 0);
    let game = server::game_api::execute(
        game,
        advance_action(Advance::Storage, ResourcePile::ideas(2)),
        0,
    );
    let displayed = server::web_view::view(&game, Some(0));
    assert!(displayed["choiceDecision"]["preview"]["name"].is_string());
    assert!(
        displayed["choiceDecision"]["preview"]["rules"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
}
