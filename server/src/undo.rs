use crate::action::{Action, after_action};
use crate::game::Game;
use crate::log::{current_turn_log, current_turn_log_mut};
use crate::movement::execute_movement_action;
use json_patch::{PatchOperation, patch};
use serde_json::Value;

const IGNORE_PATHS: [&str; 2] = ["/log/", "/log_index"];

pub(crate) fn clean_patch(mut patch: Vec<PatchOperation>) -> Vec<PatchOperation> {
    patch.retain(|op| {
        IGNORE_PATHS
            .iter()
            .all(|ignore| !op.path().as_str().starts_with(ignore))
    });
    patch
}

pub(crate) fn undo(mut game: Game) -> Result<Game, String> {
    game.log_index -= 1;
    let l = &mut current_turn_log_mut(&mut game).actions;
    let Some(i) = l.iter().rposition(|a| !a.undo.is_empty()) else {
        return Err("No undoable action".to_string());
    };

    let item = l.get_mut(i).expect("should have undoable action");
    item.log.clear();
    item.items.clear();
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
        game.context,
    ))
}

pub(crate) fn to_serde_value(game: &Game) -> Value {
    let s = serde_json::to_string(&game.cloned_data()).expect("game should be serializable");
    serde_json::from_str(&s).expect("game should be serializable")
}

pub fn redo(game: &mut Game, player_index: usize) -> Result<(), String> {
    let copy = current_turn_log(game).last_action(game).clone();
    game.log_index += 1;

    let a = copy.action;
    match a {
        Action::Playing(action) => action.execute(game, player_index, true),
        Action::Movement(action) => execute_movement_action(game, action, player_index),
        Action::Response(action) => action.redo(game, player_index),
        _ => return Err(format!("{a:?} can't be redone")),
    }?;
    after_action(game, player_index);
    Ok(())
}
