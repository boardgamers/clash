use super::*;
use crate::action::Action;
use crate::cache::Cache;
use crate::events::EventOrigin;
use crate::game::GameContext;
use crate::game_data::GameData;
use crate::log::{ActionLogAge, ActionLogItem, ActionLogRound, current_turn_log_mut};
use crate::playing_actions::PlayingAction;
use crate::wonder::Wonder;

fn game() -> Game {
    let data: GameData = serde_json::from_str(include_str!(
        "../../../tests/test_games/tactics_cards/heavy_resistance.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let mut game = Game::from_data(data, cache, GameContext::Play);
    game.wonderful_eligibility_from = Some(0);
    game.log = vec![ActionLogAge {
        age: 1,
        rounds: vec![ActionLogRound {
            round: 3,
            turns: vec![ActionLogTurn {
                turn_type: TurnType::Player(0),
                actions: vec![],
            }],
        }],
    }];
    game.log_index = 0;
    game
}

fn wonder_action(player: usize) -> ActionLogAction {
    let mut action = ActionLogAction::new(Action::Playing(PlayingAction::EndTurn), player, None, 0);
    action.items.push(ActionLogItem::new(
        player,
        ActionLogEntry::HandCard {
            card: HandCard::Wonder(Wonder::Pyramids),
            from: HandCardLocation::Hand(player),
            to: HandCardLocation::PlayToKeep,
        },
        EventOrigin::Ability("Build Wonder".into()),
        vec![],
    ));
    action
}

fn build(game: &mut Game, player: usize) {
    let action = wonder_action(player);
    current_turn_log_mut(game).actions.push(action);
    game.log_index += 1;
}

fn round_with_build(round: u32, player: usize) -> ActionLogRound {
    ActionLogRound {
        round,
        turns: vec![ActionLogTurn {
            turn_type: TurnType::Player(player),
            actions: vec![wonder_action(player)],
        }],
    }
}

fn empty_round(round: u32, player: usize) -> ActionLogRound {
    ActionLogRound {
        round,
        turns: vec![ActionLogTurn {
            turn_type: TurnType::Player(player),
            actions: vec![],
        }],
    }
}

#[test]
fn only_builder_in_last_round_qualifies_even_with_multiple_wonders() {
    let mut game = game();
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    assert!(!check(&game, game.player(0)));
    build(&mut game, 0);
    assert!(check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));
    build(&mut game, 0);
    assert!(check(&game, game.player(0)));
    build(&mut game, 1);
    assert!(!check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));
}

#[test]
fn cross_round_same_age_disqualifies_both_builders() {
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    for earlier_round in [1, 2] {
        let mut game = game();
        game.log = vec![ActionLogAge {
            age: 1,
            rounds: vec![round_with_build(earlier_round, 1), round_with_build(3, 0)],
        }];
        game.log_index = 1;
        // Corrected: B built in R1/R2 and A built in R3 of the same Age → neither qualifies.
        assert!(!check(&game, game.player(0)));
        assert!(!check(&game, game.player(1)));
        // Legacy boundary retains the old last-round-only behavior.
        game.wonderful_eligibility_from = None;
        assert!(check(&game, game.player(0)));
        assert!(!check(&game, game.player(1)));
    }
}

#[test]
fn earlier_round_sole_builder_qualifies_in_status_phase() {
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    for earlier_round in [1, 2] {
        let mut game = game();
        game.log = vec![ActionLogAge {
            age: 1,
            rounds: vec![round_with_build(earlier_round, 0), empty_round(3, 0)],
        }];
        game.log_index = 0;
        // Corrected: A built in R1/R2, nobody built in R3 → A qualifies.
        assert!(check(&game, game.player(0)));
        assert!(!check(&game, game.player(1)));
        // Legacy boundary retains the old last-round-only behavior.
        game.wonderful_eligibility_from = None;
        assert!(!check(&game, game.player(0)));
    }
}

#[test]
fn previous_age_isolation_allows_sole_builder_in_current_age() {
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    let mut game = game();
    game.log = vec![
        ActionLogAge {
            age: 1,
            rounds: vec![round_with_build(2, 1)],
        },
        ActionLogAge {
            age: 2,
            rounds: vec![round_with_build(1, 0), empty_round(3, 0)],
        },
    ];
    game.log_index = 0;
    assert!(check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));
}

#[test]
fn undo_buffer_safety_ignores_undone_wonder_in_redo_buffer() {
    let objective = magnificent_culture();
    let check = objective.status_phase_check.unwrap();
    let mut game = game();
    // Player 0 built in round 1; Player 1 built in round 3 then undid it (stays in redo buffer).
    game.log = vec![ActionLogAge {
        age: 1,
        rounds: vec![round_with_build(1, 0), round_with_build(3, 1)],
    }];
    game.log_index = 0;
    assert!(check(&game, game.player(0)));
    assert!(!check(&game, game.player(1)));

    // If only an undone Wonder exists in the current Age, nobody qualifies.
    game.log = vec![ActionLogAge {
        age: 1,
        rounds: vec![round_with_build(3, 0)],
    }];
    game.log_index = 0;
    assert!(!check(&game, game.player(0)));
}

#[test]
fn first_construction_uses_history_not_current_ownership() {
    let mut game = game();
    build(&mut game, 0);
    let builders = |g: &Game| {
        wonder_builders(
            g.log
                .iter()
                .flat_map(|a| &a.rounds)
                .flat_map(|r| &r.turns)
                .flat_map(|t| &t.actions),
        )
    };
    assert_eq!(builders(&game), [0]);
    // No Wonder remains on the board; its construction still counts.
    assert!(game.players.iter().all(|p| p.wonders_built.is_empty()));
    build(&mut game, 1);
    assert_eq!(builders(&game), [0, 1]);
}

#[test]
fn immediate_sole_builder_qualifies_until_another_player_builds() {
    use crate::content::persistent_events::{EventResponse, PersistentEventType};
    use crate::resource_pile::ResourcePile;
    for (boundary, earlier_builder, expected) in [
        // Corrected rules: 1st and 2nd Wonder by sole builder qualify; after opponent builds, false.
        (Some(0), None, true),
        (Some(0), Some(0), true),
        (Some(0), Some(1), false),
        // Legacy boundary: only the 1st Wonder in the game qualified.
        (Some(usize::MAX), None, true),
        (Some(usize::MAX), Some(0), false),
        (Some(usize::MAX), Some(1), false),
    ] {
        let data: GameData = serde_json::from_str(include_str!(
            "../../../tests/test_games/wonders/pyramids.json"
        ))
        .unwrap();
        let cache = Cache::new(&data.options);
        let mut game = Game::from_data(data, cache, GameContext::Play);
        game.wonderful_eligibility_from = boundary;
        if let Some(player) = earlier_builder {
            build(&mut game, player);
        }
        let game = crate::game_api::execute(
            game,
            Action::Playing(PlayingAction::WonderCard(Wonder::Pyramids)),
            0,
        );
        let game = crate::game_api::execute(
            game,
            Action::Response(EventResponse::Payment(vec![ResourcePile::new(
                2, 3, 6, 0, 1, 0, 5,
            )])),
            0,
        );
        let offered = game.events.iter().any(|event| matches!(&event.event_type,
            PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Magnificent Culture")));
        assert_eq!(
            offered, expected,
            "boundary={boundary:?}, earlier_builder={earlier_builder:?}"
        );
    }
}

#[test]
fn legacy_upgrade_undo_and_branch_preserve_wonderful_boundary() {
    use crate::content::persistent_events::{EventResponse, PersistentEventType};
    use crate::resource_pile::ResourcePile;
    let reload = |g: Game| {
        let json = serde_json::to_string(&crate::game_data::data(g)).unwrap();
        let data: GameData = serde_json::from_str(&json).unwrap();
        let cache = Cache::new(&data.options);
        Game::from_data(data, cache, GameContext::Play)
    };
    let data: GameData = serde_json::from_str(include_str!(
        "../../../tests/test_games/wonders/pyramids.json"
    ))
    .unwrap();
    let cache = Cache::new(&data.options);
    let mut legacy = Game::from_data(data, cache, GameContext::Play);
    // Simulate a legacy save where player 0 already built a Wonder and boundary was unset.
    build(&mut legacy, 0);
    legacy.wonderful_eligibility_from = None;
    let initial_len = crate::game_api::log_length(&legacy);
    // First new action on an unversioned live save adopts the boundary at next action.
    let after_play = crate::game_api::execute(
        reload(legacy),
        Action::Playing(PlayingAction::WonderCard(Wonder::Pyramids)),
        0,
    );
    assert_eq!(after_play.wonderful_eligibility_from, Some(initial_len + 1));
    let after_pay = crate::game_api::execute(
        after_play,
        Action::Response(EventResponse::Payment(vec![ResourcePile::new(
            2, 3, 6, 0, 1, 0, 5,
        )])),
        0,
    );
    let offered = after_pay.events.iter().any(|event| matches!(&event.event_type,
        PersistentEventType::SelectObjectives(info) if info.shown_objective.as_deref() == Some("Magnificent Culture")));
    assert!(offered);
    // Undoing must not wipe wonderful_eligibility_from.
    let mut undone = crate::game_api::execute(after_pay, Action::Undo, 0);
    assert_eq!(undone.wonderful_eligibility_from, Some(initial_len + 1));
    // Branching after undo clamps any future boundary down to the new action's log position.
    undone.wonderful_eligibility_from = Some(usize::MAX);
    let branched = crate::game_api::execute(
        undone,
        Action::Response(EventResponse::Payment(vec![ResourcePile::new(
            2, 3, 6, 0, 1, 0, 5,
        )])),
        0,
    );
    assert_eq!(
        branched.wonderful_eligibility_from,
        Some(crate::game_api::log_length(&branched))
    );
}
