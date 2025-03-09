use crate::action::{add_log_item_from_action, execute_movement_action, Action};
use crate::game::Game;
use crate::player::Player;
use crate::resource_pile::ResourcePile;
use json_patch::{patch, PatchOperation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::resource::check_for_waste;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandUndoInfo {
    pub player: usize,
    pub info: HashMap<String, String>,
}

impl CommandUndoInfo {
    #[must_use]
    pub fn new(player: &Player) -> Self {
        Self {
            info: player.event_info.clone(),
            player: player.index,
        }
    }

    pub fn apply(&self, game: &mut Game, mut undo: CommandContext) {
        let player = game.get_player_mut(self.player);
        for (k, v) in undo.info.clone() {
            player.event_info.insert(k, v);
        }

        if undo.info != self.info || !undo.gained_resources.is_empty() {
            undo.info.clone_from(&self.info);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CommandContext {
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub info: HashMap<String, String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourcePile::is_empty")]
    pub gained_resources: ResourcePile,
}

impl CommandContext {
    #[must_use]
    pub fn new(info: HashMap<String, String>) -> Self {
        Self {
            info,
            gained_resources: ResourcePile::empty(),
        }
    }
}

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

    let option = game
        .action_log
        .iter()
        .rposition(|a| !a.undo.is_empty())
        .expect("should have undoable action");

    let item = game
        .action_log
        .get_mut(option)
        .expect("should have undoable action");
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
    let copy = game.action_log[game.action_log_index].clone();
    add_log_item_from_action(game, &copy.action);
    match &game.action_log[game.action_log_index].action.clone() {
        Action::Playing(action) => action.clone().execute(game, player_index),
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
