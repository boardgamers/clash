use crate::assets::Assets;
use crate::client_state::{CameraMode, RenderResult, State};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderStage {
    Map,
    UI,
    Tooltip,
}

impl RenderStage {
    pub(crate) fn is_main(self) -> bool {
        !self.is_tooltip()
    }

    pub(crate) fn is_ui(self) -> bool {
        self == RenderStage::UI || self == RenderStage::Tooltip
    }

    pub(crate) fn is_map(self) -> bool {
        self == RenderStage::Map || self == RenderStage::Tooltip
    }

    pub(crate) fn is_tooltip(self) -> bool {
        self == RenderStage::Tooltip
    }
}

pub(crate) struct RenderContext<'a> {
    pub game: &'a Game,
    pub state: &'a State,
    pub shown_player: &'a Player,
    pub camera_mode: CameraMode,
    pub stage: RenderStage,
}

impl RenderContext<'_> {
    pub(crate) fn assets(&self) -> &Assets {
        &self.state.assets
    }

    pub(crate) fn with_camera(
        &self,
        mode: CameraMode,
        f: impl FnOnce(&RenderContext) -> RenderResult + Sized,
    ) -> RenderResult {
        let next = RenderContext {
            game: self.game,
            state: self.state,
            shown_player: self.shown_player,
            camera_mode: mode,
            stage: self.stage,
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
    pub(crate) fn world_to_screen(&self, point: Vec2) -> Vec2 {
        match self.camera_mode {
            CameraMode::Screen => point,
            CameraMode::World => self.state.camera.world_to_screen(point),
        }
    }

    #[must_use]
    pub(crate) fn screen_to_world(&self, point: Vec2) -> Vec2 {
        match self.camera_mode {
            CameraMode::Screen => point,
            CameraMode::World => self.state.camera.screen_to_world(point),
        }
    }

    pub(crate) fn can_play_action_for_player(
        &self,
        action: &PlayingActionType,
        player: usize,
    ) -> bool {
        self.can_play_action(action) && self.game.active_player() == player
    }

    pub(crate) fn can_play_action(&self, action: &PlayingActionType) -> bool {
        self.can_control_shown_player()
            && action
                .is_available(self.game, self.shown_player.index)
                .is_ok()
    }

    pub(crate) fn can_control_shown_player(&self) -> bool {
        self.can_control_active_player() && self.shown_player_is_active()
    }

    pub(crate) fn can_control_active_player(&self) -> bool {
        self.state.control_player == Some(self.game.active_player())
    }

    pub(crate) fn shown_player_is_active(&self) -> bool {
        self.game.active_player() == self.state.show_player
    }

    pub(crate) fn new_payment<T: Clone>(
        &self,
        cost: &PaymentOptions,
        value: T,
        name: &str,
        optional: bool,
    ) -> Payment<T> {
        let available = &self.shown_player.resources;
        Payment::new(cost, available, value, name, optional)
    }

    pub(crate) fn player_color(&self, player_index: usize) -> Color {
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

    pub(crate) fn mouse_pos(&self) -> Vec2 {
        self.screen_to_world(mouse_position().into())
    }

    pub(crate) fn draw_text(&self, text: &str, x: f32, y: f32) {
        if self.stage.is_main() {
            self.state.draw_text(text, x, y);
        }
    }

    pub(crate) fn draw_text_with_color(&self, text: &str, x: f32, y: f32, color: Color) {
        if self.stage.is_main() {
            self.state.draw_text_with_color(text, x, y, color);
        }
    }

    pub(crate) fn draw_rectangle(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        if self.stage.is_main() {
            macroquad::prelude::draw_rectangle(x, y, w, h, color);
        }
    }

    pub(crate) fn draw_rectangle_lines(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        thickness: f32,
        color: Color,
    ) {
        if self.stage.is_main() {
            macroquad::prelude::draw_rectangle_lines(x, y, w, h, thickness, color);
        }
    }

    pub(crate) fn draw_circle(&self, x: f32, y: f32, r: f32, color: Color) {
        if self.stage.is_main() {
            macroquad::prelude::draw_circle(x, y, r, color);
        }
    }

    pub(crate) fn draw_circle_lines(&self, x: f32, y: f32, r: f32, thickness: f32, color: Color) {
        if self.stage.is_main() {
            macroquad::prelude::draw_circle_lines(x, y, r, thickness, color);
        }
    }
}
