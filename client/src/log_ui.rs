use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, State, StateUpdate};
use crate::layout_ui::bottom_center_texture;
use crate::render_context::RenderContext;
use macroquad::math::vec2;
use server::log::TurnType;

#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    pub age: Option<String>,
    pub round: Option<String>,
    pub name: String,
    pub message: String,
}

impl LogEntry {
    pub(crate) fn new(age: Option<u32>, round: Option<u32>, name: &str, message: String) -> Self {
        LogEntry {
            age: age.map(|a| a.to_string()),
            round: round.map(|r| r.to_string()),
            name: name.to_string(),
            message,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LogDialog {
    pub lines_per_page: usize,
    pub pages: usize,
    pub current_page: usize,
    pub log_entries: Vec<LogEntry>,
}

impl LogDialog {
    pub(crate) fn new(rc: &RenderContext) -> Self {
        let lines_per_page = (rc.state.screen_size.y - 150.) as usize / 25;

        // Build structured log entries with age, round, player, and message
        let mut log_entries = Vec::new();

        for age in &rc.game.log {
            for round in &age.rounds {
                for turn in &round.turns {
                    for action in &turn.actions {
                        for message in &action.log {
                            // Simulate multiline labels to get accurate line count
                            multiline_label(
                                rc.state,
                                message,
                                Self::max_width(rc),
                                |label: &str| {
                                    log_entries.push(match turn.turn_type {
                                        TurnType::Player(p) => LogEntry::new(
                                            Some(age.age),
                                            Some(round.round),
                                            &rc.game.player_name(p),
                                            label.to_string(),
                                        ),
                                        TurnType::Setup => LogEntry::new(
                                            None,
                                            None,
                                            "Game Started",
                                            label.to_string(),
                                        ),
                                        TurnType::StatusPhase => LogEntry::new(
                                            None,
                                            Some(round.round),
                                            "Status Phase",
                                            label.to_string(),
                                        ),
                                    });
                                },
                            );
                        }
                    }
                }
            }
        }

        let total_lines = log_entries.len();
        let pages = if total_lines == 0 {
            1
        } else {
            total_lines.div_ceil(lines_per_page)
        };

        LogDialog {
            lines_per_page,
            pages,
            current_page: pages - 1,
            log_entries,
        }
    }

    fn max_width(rc: &RenderContext) -> f32 {
        rc.state.screen_size.x - 300.
    }
}

pub(crate) fn show_log(rc: &RenderContext, d: &LogDialog) -> RenderResult {
    let state = &rc.state;

    // Use the pre-calculated log entries
    let start = d.current_page * d.lines_per_page;
    let end = usize::min(start + d.lines_per_page, d.log_entries.len());
    let mut y = 0.;

    // Draw column headers
    let age_pos = vec2(30., 20.);
    let round_pos = vec2(80., 20.);
    let player_pos = vec2(140., 20.);
    let message_pos = vec2(280., 20.);

    rc.draw_text("Age", age_pos.x, age_pos.y);
    rc.draw_text("Round", round_pos.x, round_pos.y);
    rc.draw_text("Player", player_pos.x, player_pos.y);
    rc.draw_text("Message", message_pos.x, message_pos.y);

    y += 1.5; // Add some space after headers

    // Track previous values to only show when they change
    let mut prev_age: Option<String> = None;
    let mut prev_round: Option<String> = None;
    let mut prev_name: Option<String> = None;

    for entry in &d.log_entries[start..end] {
        let message_text = &entry.message;

        // Calculate positions for each column
        let age_pos = vec2(30., y * 25. + 20.);
        let round_pos = vec2(80., y * 25. + 20.);
        let player_pos = vec2(140., y * 25. + 20.);
        let message_pos = vec2(280., y * 25. + 20.);

        // Draw each column
        if let Some(age) = &entry.age
            && entry.age != prev_age
        {
            rc.draw_text(age, age_pos.x, age_pos.y);
        }
        if let Some(round) = &entry.round
            && entry.round != prev_round
        {
            rc.draw_text(round, round_pos.x, round_pos.y);
        }
        if Some(&entry.name) != prev_name.as_ref() {
            rc.draw_text(&entry.name, player_pos.x, player_pos.y);
        }
        rc.draw_text(message_text, message_pos.x, message_pos.y);

        // Update previous values
        prev_age.clone_from(&entry.age);
        prev_round.clone_from(&entry.round);
        prev_name = Some(entry.name.clone());

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
