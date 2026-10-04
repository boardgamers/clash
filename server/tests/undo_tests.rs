use common::{JsonTest, move_action, payment_response};
use server::action::{Action, execute_action};
use server::collect::{Collect, PositionCollection};
use server::content::custom_actions::PlayingActionModifier::FreeEconomyCollect;
use server::game::Game;
use server::game_api;
use server::log::{LogSliceOptions, current_turn_log_mut};
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

fn journal(game: &Game) -> Vec<Action> {
    game_api::log_slice(
        game,
        &LogSliceOptions {
            player: Some(0),
            start: 0,
            end: None,
        },
    )
}

fn collect(action_type: PlayingActionType) -> Action {
    Action::Playing(PlayingAction::Collect(Collect::new(
        Position::from_offset("C2"),
        vec![PositionCollection::new(
            Position::from_offset("B1"),
            ResourcePile::ore(1),
        )],
        action_type,
    )))
}

#[test]
fn undone_collect_does_not_block_free_economy() {
    let start = JsonTest::new("advances").load_game("collect_free_economy");
    let free = FreeEconomyCollect.playing_action_type();
    assert!(free.is_available(&start, 0).is_ok());
    let game = execute_action(start.clone(), collect(PlayingActionType::Collect), 0);
    assert!(free.is_available(&game, 0).is_err());
    let mut game = execute_action(game, Action::Undo, 0);
    assert!(free.is_available(&game, 0).is_ok());
    assert_eq!(game_api::log_length(&game), game_api::log_length(&start));

    game = execute_action(game, collect(free.clone()), 0);
    assert!(!game.can_redo());
    assert_eq!(current_turn_log_mut(&mut game).actions.len(), 1);
    game = execute_action(game, payment_response(ResourcePile::mood_tokens(1)), 0);
    assert_eq!(game.actions_left, start.actions_left);
    assert_eq!(
        game.players[0].resources.mood_tokens,
        start.players[0].resources.mood_tokens - 1
    );
    assert!(free.is_available(&game, 0).is_err());

    game = execute_action(game, Action::Undo, 0);
    game = execute_action(game, Action::Undo, 0);
    assert!(free.is_available(&game, 0).is_ok());
    assert_eq!(game.players[0].resources, start.players[0].resources);
    assert_eq!(game.actions_left, start.actions_left);
}

#[test]
fn repeated_undo_redo_and_branching_keep_the_correct_action_log() {
    let start = JsonTest::new("advances").load_game("collect_free_economy");
    let action = collect(PlayingActionType::Collect);
    let mut game = execute_action(start.clone(), action.clone(), 0);
    game = execute_action(game, action.clone(), 0);
    let expected = game.clone();
    game = execute_action(game, Action::Undo, 0);
    game = execute_action(game, Action::Undo, 0);
    assert_eq!(game_api::log_length(&game), game_api::log_length(&start));
    game = execute_action(game, Action::Redo, 0);
    let actions = &current_turn_log_mut(&mut game).actions;
    assert!(!actions[0].items.is_empty());
    assert!(actions[1].items.is_empty());
    assert_eq!(
        game_api::log_length(&game),
        game_api::log_length(&start) + 1
    );
    game = execute_action(game, Action::Redo, 0);
    assert_eq!(game.players[0].resources, expected.players[0].resources);
    assert_eq!(game.actions_left, expected.actions_left);
    assert_eq!(
        game.players[0].cities[1].mood_state,
        expected.players[0].cities[1].mood_state
    );

    game = execute_action(game, Action::Undo, 0);
    game = execute_action(game, action, 0);
    assert!(!game.can_redo());
    assert_eq!(current_turn_log_mut(&mut game).actions.len(), 2);
    game = execute_action(game, Action::Undo, 0);
    game = execute_action(game, Action::Undo, 0);
    assert_eq!(game.players[0].resources, start.players[0].resources);
    assert_eq!(game.actions_left, start.actions_left);
}

#[test]
fn journal_request_with_pre_undo_cursor_is_empty_after_movement_and_founding() {
    for (game, action) in [
        (
            JsonTest::new("movement").load_game("ship_disembark"),
            move_action(vec![1, 2], Position::from_offset("B3")),
        ),
        (
            JsonTest::new("base").load_game("found_city"),
            Action::Playing(PlayingAction::FoundCity { settler: 4 }),
        ),
    ] {
        let original_log = journal(&game);
        let game = execute_action(game, action.clone(), 0);
        // BGS records this cursor before undo, then requests new entries after saving.
        let cursor = game_api::log_length(&game);
        assert_eq!(cursor, original_log.len() + 1);
        let game = execute_action(game, Action::Undo, 0);
        assert_eq!(journal(&game), original_log);
        assert!(
            game_api::log_slice(
                &game,
                &LogSliceOptions {
                    player: Some(0),
                    start: cursor,
                    end: None,
                },
            )
            .is_empty()
        );

        let game = execute_action(game, Action::Redo, 0);
        assert_eq!(
            game_api::log_slice(
                &game,
                &LogSliceOptions {
                    player: Some(0),
                    start: original_log.len(),
                    end: None,
                },
            ),
            vec![action]
        );
    }
}

#[test]
fn journal_slices_preserve_inclusive_end_and_handle_stale_ranges() {
    let game = JsonTest::new("advances").load_game("collect_free_economy");
    let game = execute_action(game, collect(PlayingActionType::Collect), 0);
    let game = execute_action(game, collect(PlayingActionType::Collect), 0);
    let log = journal(&game);
    assert!(log.len() >= 2);
    let slice = |start, end| {
        game_api::log_slice(
            &game,
            &LogSliceOptions {
                player: Some(0),
                start,
                end,
            },
        )
    };
    assert_eq!(slice(0, None), log);
    assert_eq!(slice(0, Some(0)), log[..1]);
    assert_eq!(slice(1, Some(1)), log[1..2]);
    assert_eq!(slice(1, Some(usize::MAX)), log[1..]);
    assert!(slice(log.len(), None).is_empty());
    assert!(slice(log.len() + 1, None).is_empty());
    assert!(slice(usize::MAX, Some(usize::MAX)).is_empty());
    assert!(slice(2, Some(0)).is_empty());
}
