use crate::client_state::StateUpdate;
use crate::render_context::RenderContext;
use macroquad::math::vec2;

pub fn show_log(rc: &RenderContext) -> StateUpdate {
    let state = rc.state;
    let mut y = state.log_scroll;

    for l in &rc.game.log {
        multiline_label(l, 90, |label: &str| {
            let p = vec2(30., y * 25. + 20.);
            y += 1.;
            state.draw_text(label, p.x, p.y);
        });
    }
    StateUpdate::None
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
