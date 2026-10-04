use crate::common::{TestAction, move_action};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::EventResponse;
use server::position::Position;

mod common;

const JSON: JsonTest = JsonTest::new("tactics_cards");

#[test]
fn test_peltasts() {
    JSON.test(
        "peltasts",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            ),
        ],
    );
}

#[test]
fn test_encircled() {
    JSON.test(
        "encircled",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    2,
                )])),
            ),
        ],
    );
}

#[test]
fn test_wedge_formation() {
    JSON.test(
        "wedge_formation",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    5,
                )])),
            ),
        ],
    );
}

#[test]
fn test_high_morale() {
    JSON.test(
        "high_morale",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    6,
                )])),
            ),
        ],
    );
}

#[test]
fn test_heavy_resistance() {
    JSON.test(
        "heavy_resistance",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    7,
                )])),
            ),
        ],
    );
}

#[test]
fn heavy_resistance_can_resolve_a_battle_at_a_wonder_city() {
    use server::content::custom_actions::CustomActionType;
    use server::game_api;
    use server::wonder::Wonder;

    let mut game = JSON.load_game("heavy_resistance");
    let city = Position::from_offset("C1");
    let wonders = vec![Wonder::GreatLibrary, Wonder::GreatLighthouse];
    game.players[0].action_cards.clear();
    game.players[1].action_cards = vec![7];
    game.players[1].units.truncate(1);
    game.players[1].get_city_mut(city).pieces.wonders = wonders.clone();
    game.players[1].wonders_built = wonders.clone();
    // Two strong attacker rolls, then a low defender roll. Heavy Resistance
    // applies normally; resolving the round must also transfer the wonders.
    game.dice_roll_outcomes = vec![0, 11, 11];
    let game = game.clone(); // initialize ownership and wonder abilities
    let game = game_api::execute(game, move_action(vec![0, 2], city), 0);
    let game = game_api::execute(
        game,
        Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
            7,
        )])),
        1,
    );
    assert!(game.player(1).try_get_city(city).is_none());
    assert_eq!(game.player(0).get_city(city).pieces.wonders, wonders);
    assert_eq!(
        game.player(1).wonders_built,
        wonders,
        "builder credit is retained"
    );
    for wonder in &wonders {
        assert!(game.player(0).wonders_owned.contains(*wonder));
        assert!(!game.player(1).wonders_owned.contains(*wonder));
    }
    for ability in [
        CustomActionType::GreatLibrary,
        CustomActionType::GreatLighthouse,
    ] {
        assert!(game.player(0).special_actions.contains_key(
            &server::content::custom_actions::SpecialAction::Custom(ability)
        ));
        assert!(!game.player(1).special_actions.contains_key(
            &server::content::custom_actions::SpecialAction::Custom(ability)
        ));
    }
    let restored = game.clone();
    assert_eq!(
        restored.player(0).wonders_owned,
        game.player(0).wonders_owned
    );
    assert_eq!(
        restored.player(1).wonders_owned,
        game.player(1).wonders_owned
    );
}

#[test]
fn test_high_ground() {
    JSON.test(
        "high_ground",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    9,
                )])),
            ),
        ],
    );
}

#[test]
fn test_surprise() {
    JSON.test(
        "surprise",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1")))
                .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    10,
                )])),
            ),
        ],
    );
}

#[test]
fn test_siege() {
    JSON.test(
        "siege",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    11,
                )])),
            ),
        ],
    );
}

#[test]
fn test_scout() {
    JSON.test(
        "scout",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    1,
                )])),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    23,
                )])),
            ),
        ],
    );
}

#[test]
fn test_martyr() {
    JSON.test(
        "martyr",
        vec![
            TestAction::undoable(0, move_action(vec![7, 8], Position::from_offset("D2")))
                .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    6,
                )])),
            )
            .skip_json(),
            TestAction::not_undoable(
                1,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    24,
                )])),
            )
            .skip_json(),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![7])))
                .skip_json(),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![2]))),
        ],
    );
}

#[test]
fn test_martyr2() {
    JSON.test(
        "martyr2",
        vec![
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![10])))
                .skip_json(),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectUnits(vec![12]))),
        ],
    );
}

#[test]
fn test_archers() {
    JSON.test(
        "archers",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    26,
                )])),
            )
            .skip_json(),
            TestAction::not_undoable(1, Action::Response(EventResponse::SelectUnits(vec![1]))),
        ],
    );
}

#[test]
fn test_flanking() {
    JSON.test(
        "flanking",
        vec![
            TestAction::undoable(0, move_action(vec![0], Position::from_offset("C1"))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![HandCard::ActionCard(
                    42,
                )])),
            ),
        ],
    );
}
