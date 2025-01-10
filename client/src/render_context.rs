use crate::assets::Assets;
use crate::client_state::{ActiveDialog, CameraMode, State, StateUpdate};
use macroquad::camera::set_default_camera;
use macroquad::math::{bool, Vec2};
use macroquad::prelude::set_camera;
use server::game::Game;
use server::player::Player;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone)]
pub struct ShownPlayer {
    pub index: usize,
    pub is_active: bool,
    pub can_control_active_player: bool,
    pub can_control: bool,
    pub can_play_action: bool,
}

impl ShownPlayer {
    #[must_use]
    pub fn get<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.index)
    }
}

pub struct RenderContext<'a> {
    pub shown_player: ShownPlayer,
    pub game: &'a Game,
    pub state: &'a State,
    pub player: &'a Player, // the player that is being shown
    pub camera_mode: CameraMode,
}

impl RenderContext<'_> {
    pub fn assets(&self) -> &Assets {
        &self.state.assets
    }

    pub fn with_camera(
        &self,
        mode: CameraMode,
        f: impl FnOnce(&RenderContext) -> StateUpdate + Sized,
    ) -> StateUpdate {
        let next = RenderContext {
            shown_player: self.shown_player.clone(),
            game: self.game,
            state: self.state,
            player: self.player,
            camera_mode: mode,
        };
        next.set_camera();
        let update = f(&next);
        self.set_camera();
        update
    }

    fn set_camera(&self) {
        match self.camera_mode {
            CameraMode::Screen => set_default_camera(),
            CameraMode::World => set_camera(&self.state.camera),
        };
    }

    #[must_use]
    pub fn world_to_screen(&self, point: Vec2) -> Vec2 {
        match self.camera_mode {
            CameraMode::Screen => point,
            CameraMode::World => self.state.camera.world_to_screen(point),
        }
    }

    #[must_use]
    pub fn screen_to_world(&self, point: Vec2) -> Vec2 {
        match self.camera_mode {
            CameraMode::Screen => point,
            CameraMode::World => self.state.camera.screen_to_world(point),
        }
    }
}
