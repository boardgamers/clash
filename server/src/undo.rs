use crate::action::{add_log_item_from_action, execute_movement_action, Action};
use crate::game::Game;
use crate::log::{current_player_turn_log, current_player_turn_log_mut};
use crate::resource::check_for_waste;
use json_patch::{patch, PatchOperation};
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

pub(crate) fn undo(mut game: Game) -> Game {
    game.action_log_index -= 1;
    game.log.remove(game.log.len() - 1);

    let l = &mut current_player_turn_log_mut(&mut game).items;
    let option = l
        .iter()
        .rposition(|a| !a.undo.is_empty())
        .expect("should have undoable action");

    let item = l.get_mut(option).expect("should have undoable action");
    let p = std::mem::take(&mut item.undo);

    match &item.action {
        Action::Undo => panic!("undo action can't be undone"),
        Action::Redo => panic!("redo action can't be undone"),
        _ => {}
    }

    let mut v = to_serde_value(&game);

    patch(&mut v, &p).expect("could not patch game data");

    Game::from_data(serde_json::from_value(v).expect("should be able to deserialize game"))
}

pub(crate) fn to_serde_value(game: &Game) -> Value {
    let s = serde_json::to_string(&game.cloned_data()).expect("game should be serializable");
    serde_json::from_str(&s).expect("game should be serializable")
}

pub fn redo(game: &mut Game, player_index: usize) {
    let copy = current_player_turn_log(game).item(game).clone();
    add_log_item_from_action(game, &copy.action);
    match &current_player_turn_log(game).item(game).action.clone() {
        Action::Playing(action) => action.clone().execute(game, player_index, true),
        Action::Movement(action) => {
            execute_movement_action(game, action.clone(), player_index);
        }
        Action::Response(action) => action.clone().redo(game, player_index),
        Action::Undo => panic!("undo action can't be redone"),
        Action::Redo => panic!("redo action can't be redone"),
    }
    game.action_log_index += 1;
    check_for_waste(game);
}
