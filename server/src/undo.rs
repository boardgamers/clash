use crate::action::{Action, add_log_item_from_action, execute_movement_action, after_action};
use crate::game::Game;
use crate::log::{current_player_turn_log, current_player_turn_log_mut};
use json_patch::{PatchOperation, patch};
use serde_json::Value;

const IGNORE_PATHS: [&str; 3] = ["/action_log/", "/action_log_index", "/log/"];

pub(crate) fn clean_patch(mut patch: Vec<PatchOperation>) -> Vec<PatchOperation> {
    patch.retain(|op| {
        IGNORE_PATHS
            .iter()
            .all(|ignore| !op.path().as_str().starts_with(ignore))
    });
    patch
}

pub(crate) fn undo(mut game: Game) -> Result<Game, String> {
    game.action_log_index -= 1;
    game.log.remove(game.log.len() - 1);

    let l = &mut current_player_turn_log_mut(&mut game).items;
    let Some(i) = l.iter().rposition(|a| !a.undo.is_empty()) else {
        return Err("No undoable action".to_string());
    };

    let item = l.get_mut(i).expect("should have undoable action");
    let p = std::mem::take(&mut item.undo);

    match &item.action {
        Action::Undo => return Err("undo action can't be undone".to_string()),
        Action::Redo => return Err("redo action can't be undone".to_string()),
        _ => {}
    }

    let mut v = to_serde_value(&game);

    patch(&mut v, &p).map_err(|e| format!("Failed to apply patch: {e}"))?;

    Ok(Game::from_data(
        serde_json::from_value(v).map_err(|e| format!("Failed to deserialize game: {e}"))?,
        game.cache,
    ))
}

pub(crate) fn to_serde_value(game: &Game) -> Value {
    let s = serde_json::to_string(&game.cloned_data()).expect("game should be serializable");
    serde_json::from_str(&s).expect("game should be serializable")
}

pub fn redo(game: &mut Game, player_index: usize) -> Result<(), String> {
    let copy = current_player_turn_log(game).item(game).clone();
    game.action_log_index += 1;
    add_log_item_from_action(game, &copy.action);

    match copy.action.clone() {
        Action::Playing(action) => action.execute(game, player_index, true),
        Action::Movement(action) => execute_movement_action(game, action, player_index),
        Action::Response(action) => action.redo(game, player_index),
        Action::Undo => return Err("undo action can't be redone".to_string()),
        Action::Redo => return Err("redo action can't be redone".to_string()),
    }?;
    after_action(game, player_index);
    Ok(())
}
