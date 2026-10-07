// combat

use crate::common::{TestAction, move_action, payment_response};
use common::JsonTest;
use server::action::Action;
use server::card::HandCard;
use server::content::persistent_events::{
    EventResponse, PersistentEventRequest, PersistentEventType,
};
use server::game::Game;
use server::game_api::execute;
use server::playing_actions::PlayingAction::Recruit;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::unit::{UnitType, Units};

mod common;

const JSON: JsonTest = JsonTest::new("combat");

fn retreat_casualties_game(mixed_defenders: bool) -> Game {
    let mut game = JSON.load_game("retreat");
    game.player_mut(0).units.retain(|unit| unit.id < 4);
    for unit in &mut game.player_mut(0).units {
        unit.unit_type = UnitType::Infantry;
    }
    let defender = game.player(1).units[0].clone();
    game.player_mut(1).units = (0..4)
        .map(|id| {
            let mut unit = defender.clone();
            unit.id = id;
            unit.unit_type = if mixed_defenders && id == 3 {
                UnitType::Infantry
            } else {
                UnitType::Cavalry
            };
            unit
        })
        .collect();
    game.player_mut(1).next_unit_id = 4;
    // Four 3/Elephant faces on each side: no matching ability, 12 CV, 2 hits.
    game.dice_roll_outcomes = vec![4; 8];
    game
}

fn assert_retreat_round(game: &Game, round: u32) {
    let event = game.current_event();
    let PersistentEventType::CombatRoundEnd(end) = &event.event_type else {
        panic!("expected combat round end");
    };
    assert_eq!(serde_json::to_value(&end.phase).unwrap(), "Retreat");
    assert_eq!(end.combat.stats.round, round);
    assert_eq!(event.player.index, 0);
    assert!(matches!(
        &event.player.handler.as_ref().unwrap().request,
        PersistentEventRequest::BoolRequest(text) if text == "Do you want to retreat?"
    ));
}

#[test]
fn both_armies_lose_casualties_before_retreat_or_another_round() {
    let game = execute(
        retreat_casualties_game(false),
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
    );
    assert_retreat_round(&game, 1);
    assert_eq!(game.player(0).units.len(), 2);
    assert_eq!(game.player(1).units.len(), 2);
    let retreated = execute(game.clone(), Action::Response(EventResponse::Bool(true)), 0);
    assert!(retreated.events.is_empty());
    assert_eq!(retreated.player(0).units.len(), 2);
    assert_eq!(retreated.player(1).units.len(), 2);
    assert!(
        retreated
            .player(0)
            .units
            .iter()
            .all(|u| u.position == Position::from_offset("C2"))
    );

    let mut game = game;
    game.dice_roll_outcomes = vec![0; 4]; // No hits in round 2.
    let continued = execute(game, Action::Response(EventResponse::Bool(false)), 0);
    assert_retreat_round(&continued, 2);
    assert_eq!(continued.player(0).units.len(), 2);
    assert_eq!(continued.player(1).units.len(), 2);
}

#[test]
fn defender_chooses_casualties_before_attacker_can_retreat() {
    let game = execute(
        retreat_casualties_game(true),
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
    );
    let event = game.current_event();
    assert_eq!(event.player.index, 1);
    assert!(matches!(
        &event.player.handler.as_ref().unwrap().request,
        PersistentEventRequest::SelectUnits(_)
    ));
    assert_eq!(game.player(0).units.len(), 2);
    assert_eq!(game.player(1).units.len(), 4);
    let game = execute(
        game.clone(),
        Action::Response(EventResponse::SelectUnits(vec![0, 3])),
        1,
    );
    assert_retreat_round(&game, 1);
    assert_eq!(game.player(1).units.len(), 2);
}

#[test]
fn saved_early_retreat_answers_apply_remaining_losses_once_without_repeating_prompt() {
    let initial = retreat_casualties_game(false);
    let defenders = initial.player(1).units.clone();
    let mut game = execute(
        initial,
        move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        0,
    );
    // Recreate a save from the old engine: attacker losses resolved, defender
    // losses pending, and a retreat answer requested in the default phase.
    game.player_mut(1).units = defenders;
    let event = game.events.last_mut().unwrap();
    let PersistentEventType::CombatRoundEnd(end) = &mut event.event_type else {
        panic!("expected combat round end");
    };
    end.phase = Default::default();
    end.combat.stats.defender.losses = Units::default();
    for retreat in [true, false] {
        let mut saved = game.clone();
        saved.dice_roll_outcomes = vec![0; 4];
        let result = execute(saved, Action::Response(EventResponse::Bool(retreat)), 0);
        assert_eq!(result.player(0).units.len(), 2);
        assert_eq!(result.player(1).units.len(), 2);
        if retreat {
            assert!(result.events.is_empty());
        } else {
            assert_retreat_round(&result, 2);
        }
    }
}

#[test]
fn test_remove_casualties_attacker() {
    JSON.test(
        "remove_casualties_attacker",
        vec![
            TestAction::not_undoable(
                0,
                move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            ),
            TestAction::undoable(0, Action::Response(EventResponse::SelectUnits(vec![0, 1]))),
        ],
    );
}

#[test]
fn test_remove_casualties_defender() {
    JSON.test(
        "remove_casualties_defender",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_metallurgy() {
    JSON.test(
        "direct_capture_city_metallurgy",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_fortress() {
    JSON.test(
        "direct_capture_city_fortress",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_capture_and_raze_city() {
    JSON.test(
        "capture_and_raze_city",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_direct_capture_city_only_fortress() {
    JSON.test(
        "direct_capture_city_only_fortress",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_combat_all_modifiers() {
    JSON.test(
        "combat_all_modifiers",
        vec![
            TestAction::undoable(
                0,
                move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            )
            .skip_json(),
            TestAction::undoable(0, payment_response(ResourcePile::ore(1))).skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::Payment(vec![
                    ResourcePile::empty(),
                    ResourcePile::ore(2),
                ])),
            )
            .skip_json(),
            TestAction::not_undoable(1, payment_response(ResourcePile::ore(1))).skip_json(),
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
                    2,
                )])),
            ),
        ],
    );
}

#[test]
fn test_combat_fanaticism() {
    JSON.test(
        "combat_fanaticism",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
        )],
    );
}

#[test]
fn test_retreat() {
    JSON.test(
        "retreat",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::undoable(0, Action::Response(EventResponse::Bool(true))),
        ],
    );
}

#[test]
fn test_do_not_retreat() {
    JSON.test(
        "retreat_no",
        vec![
            TestAction::not_undoable(0, move_action(vec![0], Position::from_offset("C1"))),
            TestAction::not_undoable(0, Action::Response(EventResponse::Bool(false))),
        ],
    );
}

#[test]
fn test_ship_combat() {
    JSON.test(
        "ship_combat",
        vec![
            TestAction::not_undoable(0, move_action(vec![7, 8], Position::from_offset("D2"))),
            TestAction::not_undoable(0, Action::Response(EventResponse::SelectUnits(vec![1]))),
        ],
    );
}

#[test]
fn test_ship_combat_war_ships() {
    JSON.test(
        "ship_combat_war_ships",
        vec![TestAction::not_undoable(
            0,
            move_action(vec![7, 8], Position::from_offset("D2")),
        )],
    );
}

#[test]
fn test_recruit_combat() {
    JSON.test(
        "recruit_combat",
        vec![
            TestAction::undoable(
                0,
                Action::Playing(Recruit(server::recruit::Recruit::new(
                    &Units::new(0, 0, 4, 0, 0, None),
                    Position::from_offset("C2"),
                    ResourcePile::wood(5) + ResourcePile::gold(3),
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::mood_tokens(1))),
            )
            .skip_json(),
            TestAction::not_undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::gold(1))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            )
            .skip_json(),
            TestAction::not_undoable(0, move_action(vec![12, 13], Position::from_offset("C4")))
                .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::ResourceReward(ResourcePile::culture_tokens(
                    1,
                ))),
            )
            .skip_json(),
            TestAction::undoable(
                0,
                Action::Response(EventResponse::SelectHandCards(vec![
                    HandCard::ObjectiveCard(38),
                ])),
            ),
        ],
    );
}
