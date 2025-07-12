use crate::client_state::{NO_UPDATE, RenderResult, State};
use crate::render_context::RenderContext;
use macroquad::math::vec2;
use server::game::Game;

#[derive(Clone, Debug)]
pub(crate) struct LogDialog {
    pub log_scroll: f32,
}

impl Default for LogDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogDialog {
    pub(crate) fn new() -> Self {
        LogDialog { log_scroll: 0. }
    }
}

pub(crate) fn show_log(rc: &RenderContext, d: &LogDialog) -> RenderResult {
    draw_log(rc.game, rc.state, d.log_scroll, |label: &str, y: f32| {
        let p = vec2(30., y * 25. + 20.);
        rc.draw_text(label, p.x, p.y);
    });
    NO_UPDATE
}

pub(crate) fn get_log_end(game: &Game, state: &State, height: f32) -> f32 {
    let mut end = 0.;
    draw_log(game, state, 0., |_label: &str, y: f32| {
        end = y;
    });
    -end + (height - 40.) / 25.
}

fn draw_log(game: &Game, state: &State, start_scroll: f32, mut render: impl FnMut(&str, f32)) {
    let mut y = start_scroll;

    for l in &game.log {
        for e in l {
            multiline_label(state, e, state.screen_size.x - 100., |label: &str| {
                render(label, y);
                y += 1.;
            });
        }
    }
}

pub(crate) fn multiline_label(state: &State, label: &str, len: f32, mut print: impl FnMut(&str)) {
    let mut line = String::new();
    label.split(' ').for_each(|s| {
        let next = format!("{line} {s}");
        let dimensions = state.measure_text(&next);
        if dimensions.width > len {
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

pub(crate) struct MultilineText {
    pub width: f32,
    pub text: Vec<String>,
}

impl MultilineText {
    pub(crate) fn default() -> Self {
        MultilineText {
            width: 500.,
            text: vec![],
        }
    }

    pub(crate) fn of(rc: &RenderContext, text: &str) -> Self {
        let mut t = Self::default();
        t.add(rc, text);
        t
    }

    pub(crate) fn from(rc: &RenderContext, text: &[String]) -> Self {
        let mut t = Self::default();
        for label in text {
            t.add(rc, label);
        }
        t
    }

    pub(crate) fn add(&mut self, rc: &RenderContext, label: &str) {
        multiline_label(rc.state, label, self.width, |line: &str| {
            self.text.push(line.to_string());
        });
    }
}
