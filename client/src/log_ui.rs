use macroquad::math::vec2;
use macroquad::prelude::mouse_wheel;
use server::game::Game;

use crate::client_state::{State, StateUpdate};
use crate::render_context::RenderContext;

pub fn show_log(rc: &RenderContext) -> StateUpdate {
    let state = rc.state;
    let mut y = state.log_scroll;
    let mut label = |label: &str| {
        let p = vec2(300., y * 25. + 20.);
        y += 1.;
        state.draw_text(label, p.x, p.y);
    };

    rc.game.log.iter().for_each(|l| {
        let mut line = String::new();
        l.split(' ').for_each(|s| {
            if line.len() + s.len() > 100 {
                label(&line);
                line = String::new();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(s);
        });
        if !line.is_empty() {
            label(&line);
        }
    });
    StateUpdate::None
}
