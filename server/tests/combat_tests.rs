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
fn fanaticism_infantry_requires_a_temple_at_the_battle_city() {
    for temple in [true, false] {
        let mut game = JSON.load_game("combat_fanaticism");
        game.player_mut(1)
            .get_city_mut(Position::from_offset("C1"))
            .pieces
            .temple = temple.then_some(1);
        let game = execute(
            game,
            move_action(vec![0, 1, 2, 3, 4, 5], Position::from_offset("C1")),
            0,
        );
        // C1 is captured before Fanaticism resolves. The replacement is placed
        // in C2, which has no Temple: only the battle city needs one.
        assert_eq!(
            game.player(1)
                .units
                .iter()
                .filter(|u| u.unit_type == UnitType::Infantry)
                .count(),
            usize::from(temple),
        );
    }
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

#[test]
fn surviving_defenders_are_restricted_only_for_their_current_turn() {
    use server::movement::MovementRestriction;
    for own_turn in [false, true] {
        let mut game = execute(
            retreat_casualties_game(false),
            move_action(vec![0, 1, 2, 3], Position::from_offset("C1")),
            0,
        );
        // Resolve the same battle as an ordinary out-of-turn defense or as
        // an event battle during the defender's turn (including older saves).
        if own_turn {
            game.current_player_index = 1;
            // A unit may already have fought in another defensive battle.
            game.player_mut(1).units[0]
                .movement_restrictions
                .push(MovementRestriction::Battle);
        }
        let mut game = execute(game, Action::Response(EventResponse::Bool(true)), 0);
        assert!(game.events.is_empty());
        assert_eq!(game.player(1).units.len(), 2);
        for unit in &game.player(1).units {
            assert_eq!(
                unit.movement_restrictions
                    .iter()
                    .filter(|r| **r == MovementRestriction::Battle)
                    .count(),
                usize::from(own_turn)
            );
        }
        server::player::end_turn(&mut game, 1);
        assert!(
            game.player(1)
                .units
                .iter()
                .all(|u| u.movement_restrictions.is_empty())
        );
    }
}

#[test]
fn defending_ships_stop_but_passengers_and_longships_remain_free() {
    use server::movement::MovementRestriction;
    for longships in [false, true] {
        let mut game = JSON.load_game("ship_combat");
        let mut passenger = game.player(0).units[0].clone();
        passenger.id = 99;
        passenger.position = Position::from_offset("D2");
        passenger.carrier_id = Some(1);
        game.player_mut(1).units.push(passenger);
        if longships {
            game.player_mut(1)
                .special_advances
                .insert(serde_json::from_str("\"Longships\"").unwrap());
        }
        game.dice_roll_outcomes = vec![0; 3];
        let mut game = execute(
            game,
            move_action(vec![7, 8], Position::from_offset("D2")),
            0,
        );
        assert_retreat_round(&game, 1);
        game.current_player_index = 1;
        let game = execute(game, Action::Response(EventResponse::Bool(true)), 0);
        assert!(game.events.is_empty());
        assert_eq!(
            game.player(1)
                .get_unit(1)
                .movement_restrictions
                .contains(&MovementRestriction::Battle),
            !longships
        );
        assert!(game.player(1).get_unit(99).movement_restrictions.is_empty());
    }
}

#[test]
fn naval_defenders_exclude_passengers_from_dice_and_combat_casualties() {
    let mut game = JSON.load_game("ship_combat");
    let mut ship = game.player(1).get_unit(1).clone();
    ship.id = 2;
    game.player_mut(1).units.push(ship);
    for id in 90..94 {
        let mut passenger = game.player(0).get_unit(0).clone();
        passenger.id = id;
        passenger.position = Position::from_offset("D2");
        passenger.carrier_id = Some(if id < 92 { 1 } else { 2 });
        game.player_mut(1).units.push(passenger);
    }
    assert_eq!(server::combat_stats::active_defenders(&game, 1, Position::from_offset("D2")), vec![1, 2]);
    // Two dice per side; 3 on each die yields one hit per side.
    game.dice_roll_outcomes = vec![4; 20];
    let before = game.dice_roll_log.len();
    let mut game = execute(game, move_action(vec![7, 8], Position::from_offset("D2")), 0);
    assert_eq!(game.dice_roll_log.len() - before, 4, "only ships roll");
    // Resolve the attacker's ship loss and any excess passengers first.
    for _ in 0..4 {
        let event = game.current_event();
        if event.player.index == 1 { break; }
        let PersistentEventRequest::SelectUnits(req) = &event.player.handler.as_ref().unwrap().request else { panic!("expected casualties"); };
        let ids = req.request.choices.iter().take(*req.request.needed.start() as usize).copied().collect();
        game = execute(game, Action::Response(EventResponse::SelectUnits(ids)), 0);
    }
    // Identical ships are removed automatically; the remaining prompt is only
    // for passengers exceeding the surviving ship's two transport spaces.
    assert_eq!(game.player(1).get_units(Position::from_offset("D2")).iter().filter(|u| u.is_ship()).count(), 1);
    let event = game.current_event();
    assert_eq!(event.player.index, 1);
    let PersistentEventRequest::SelectUnits(req) = &event.player.handler.as_ref().unwrap().request else { panic!("expected excess passengers"); };
    assert_eq!(req.request.choices, vec![90, 91, 92, 93]);
    assert_eq!(req.request.needed, 2..=2);
}
