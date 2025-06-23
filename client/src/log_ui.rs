use crate::client_state::{NO_UPDATE, RenderResult};
use crate::render_context::RenderContext;
use macroquad::math::vec2;
use server::game::Game;

#[derive(Clone, Debug)]
pub struct LogDialog {
    pub log_scroll: f32,
}

impl Default for LogDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogDialog {
    pub fn new() -> Self {
        LogDialog { log_scroll: 0. }
    }
}

pub fn show_log(rc: &RenderContext, d: &LogDialog) -> RenderResult {
    draw_log(rc.game, d.log_scroll, |label: &str, y: f32| {
        let p = vec2(30., y * 25. + 20.);
        rc.draw_text(label, p.x, p.y);
    });
    NO_UPDATE
}

pub fn get_log_end(game: &Game, height: f32) -> f32 {
    let mut end = 0.;
    draw_log(game, 0., |_label: &str, y: f32| {
        end = y;
    });
    -end + (height - 40.) / 25.
}

fn draw_log(game: &Game, start_scroll: f32, mut render: impl FnMut(&str, f32)) {
    let mut y = start_scroll;

    for l in &game.log {
        for e in l {
            multiline_label(e, 90, |label: &str| {
                render(label, y);
                y += 1.;
            });
        }
    }
}

pub fn multiline_label(label: &str, len: usize, mut print: impl FnMut(&str)) {
    let mut line = String::new();
    label.split(' ').for_each(|s| {
        if line.len() + s.len() > len {
            print(&line);
            line = "    ".to_string();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(s);
    });
    if !line.is_empty() {
        print(&line);
    }
}

pub fn break_text(label: &str, len: usize, result: &mut Vec<String>) {
    multiline_label(label, len, |label: &str| {
        result.push(label.to_string());
    });
}
