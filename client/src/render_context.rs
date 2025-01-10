use macroquad::math::{bool, Vec2};
use server::game::Game;
use server::player::Player;
use crate::client_state::ActiveDialog;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
pub struct ShownPlayer {
    pub index: usize,
    pub shown_player_is_active: bool,
    pub can_control_active_player: bool,
    pub can_control: bool,
    pub can_play_action: bool,
    pub active_dialog: ActiveDialog,
    pub pending_update: bool,
    pub screen_size: Vec2,
}

impl ShownPlayer {
    #[must_use]
    pub fn get<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.index)
    }
}
