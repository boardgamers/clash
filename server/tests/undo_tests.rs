use common::{JsonTest, payment_response};
use server::action::{Action, execute_action};
use server::collect::{Collect, PositionCollection};
use server::content::custom_actions::CustomActionType::FreeEconomyCollect;
use server::game_api;
use server::log::current_turn_log_mut;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::position::Position;
use server::resource_pile::ResourcePile;

mod common;

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
    let free = PlayingActionType::Custom(FreeEconomyCollect);
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
    assert!(!actions[0].log.is_empty());
    assert!(actions[1].log.is_empty());
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
