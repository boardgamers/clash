use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, State, StateUpdate};
use crate::layout_ui::bottom_center_texture;
use crate::render_context::RenderContext;
use macroquad::math::vec2;

#[derive(Clone, Debug)]
pub(crate) struct LogDialog {
    pub lines_per_page: usize,
    pub pages: usize,
    pub current_page: usize,
    pub flattened_log: Vec<String>,
}

impl LogDialog {
    pub(crate) fn new(rc: &RenderContext) -> Self {
        let lines_per_page = (rc.state.screen_size.y - 100.) as usize / 25;

        // Flatten and simulate multiline labels to get accurate line count
        let mut flattened_log = Vec::new();

        for l in &rc.game.log {
            for e in l {
                multiline_label(rc.state, e, rc.state.screen_size.x - 100., |label: &str| {
                    flattened_log.push(label.to_string());
                });
            }
        }

        let total_lines = flattened_log.len();
        let pages = if total_lines == 0 {
            1
        } else {
            total_lines.div_ceil(lines_per_page)
        };

        LogDialog {
            lines_per_page,
            pages,
            current_page: pages - 1,
            flattened_log,
        }
    }
}

pub(crate) fn show_log(rc: &RenderContext, d: &LogDialog) -> RenderResult {
    let state = &rc.state;

    // Use the pre-calculated flattened log
    let start = d.current_page * d.lines_per_page;
    let end = usize::min(start + d.lines_per_page, d.flattened_log.len());
    let mut y = 0.;

    for line in &d.flattened_log[start..end] {
        let p = vec2(30., y * 25. + 20.);
        rc.draw_text(line, p.x, p.y);
        y += 1.;
    }
    // Bottom center navigation
    let first_offset = vec2(-160., -30.);
    let prev_offset = vec2(-90., -30.);
    let next_offset = vec2(42., -30.);
    let last_offset = vec2(112., -30.);
    let page_text = format!("Page {} / {}", d.current_page + 1, d.pages);
    // Use bottom_center_texture for navigation buttons
    let first_clicked = d.current_page > 0
        && bottom_center_texture(rc, &rc.assets().start, first_offset, "First page");
    let prev_clicked = d.current_page > 0
        && bottom_center_texture(rc, &rc.assets().undo, prev_offset, "Previous page");
    let next_clicked = d.current_page < d.pages - 1
        && bottom_center_texture(rc, &rc.assets().redo, next_offset, "Next page");
    let last_clicked = d.current_page < d.pages - 1
        && bottom_center_texture(rc, &rc.assets().end, last_offset, "Last page");
    rc.draw_text(
        &page_text,
        state.screen_size.x / 2. - 40.,
        state.screen_size.y - 60.,
    );
    // Handle clicks
    if first_clicked {
        let mut new_dialog = d.clone();
        new_dialog.current_page = 0;
        return StateUpdate::open_dialog(ActiveDialog::Log(new_dialog));
    }
    if prev_clicked {
        let mut new_dialog = d.clone();
        new_dialog.current_page -= 1;
        return StateUpdate::open_dialog(ActiveDialog::Log(new_dialog));
    }
    if next_clicked {
        let mut new_dialog = d.clone();
        new_dialog.current_page += 1;
        return StateUpdate::open_dialog(ActiveDialog::Log(new_dialog));
    }
    if last_clicked {
        let mut new_dialog = d.clone();
        new_dialog.current_page = d.pages - 1;
        return StateUpdate::open_dialog(ActiveDialog::Log(new_dialog));
    }
    NO_UPDATE
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
