use crate::assets::Assets;
use crate::client_state::{CameraMode, State, StateUpdate};
use crate::payment_ui::Payment;
use macroquad::camera::set_default_camera;
use macroquad::color::{Color, PINK, YELLOW};
use macroquad::input::mouse_position;
use macroquad::math::{Vec2, bool};
use macroquad::prelude::{BLACK, LIME, SKYBLUE, WHITE, set_camera};
use server::game::Game;
use server::payment::PaymentOptions;
use server::player::Player;
use server::playing_actions::PlayingActionType;

pub struct RenderContext<'a> {
    pub game: &'a Game,
    pub state: &'a State,
    pub shown_player: &'a Player, // the player that is being shown
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
            game: self.game,
            state: self.state,
            shown_player: self.shown_player,
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
        }
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

    pub fn can_play_action_for_player(&self, action: &PlayingActionType, player: usize) -> bool {
        self.can_play_action(action) && self.game.active_player() == player
    }
    
    pub fn can_play_action(&self, action: &PlayingActionType) -> bool {
        self.can_control_shown_player()
            && action
                .is_available(self.game, self.shown_player.index)
                .is_ok()
    }

    pub fn can_control_shown_player(&self) -> bool {
        self.can_control_active_player() && self.shown_player_is_active()
    }

    pub fn can_control_active_player(&self) -> bool {
        self.state.control_player == Some(self.game.active_player())
    }

    pub fn shown_player_is_active(&self) -> bool {
        self.game.active_player() == self.state.show_player
    }

    pub fn new_payment(&self, cost: &PaymentOptions, name: &str, optional: bool) -> Payment {
        let available = &self.shown_player.resources;
        Payment::new(cost, available, name, optional)
    }

    pub fn player_color(&self, player_index: usize) -> Color {
        let c = &self.game.player(player_index).civilization;
        if c.is_barbarian() {
            return WHITE;
        }
        if c.is_pirates() {
            return BLACK;
        }
        match player_index {
            0 => YELLOW,
            1 => PINK,
            2 => SKYBLUE,
            3 => LIME,
            _ => panic!("unexpected player index"),
        }
    }

    pub fn mouse_pos(&self) -> Vec2 {
        self.screen_to_world(mouse_position().into())
    }
}
