use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, State, StateUpdate};
use crate::layout_ui::bottom_center_texture;
use crate::render_context::RenderContext;
use macroquad::math::vec2;

#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    pub age: u32,
    pub round: u32,
    pub player_name: String,
    pub message: String,
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

        // Create a list of all player log entries with their ranges
        let mut player_log_ranges = Vec::new();

        // Iterate over action log to collect all player entries and their log ranges
        for action_log_age in &rc.game.action_log {
            for action_log_round in &action_log_age.rounds {
                for action_log_player in &action_log_round.players {
                    player_log_ranges.push((
                        action_log_age.age,
                        action_log_round.round,
                        action_log_player.index,
                        action_log_player.log_index,
                    ));
                }
            }
        }

        // Sort by log_index to ensure proper ordering
        player_log_ranges.sort_by_key(|(_, _, _, log_index)| *log_index);

        // Helper function to determine if a message should skip player name
        fn should_skip_player_name(message: &str) -> bool {
            message.starts_with("Age ") && message.contains(" has started")
                || message == "The game has started"
                || message.starts_with("Round ") && message.contains("/3")
                || message.starts_with( "The game has entered")
                || message.starts_with( " ") // multi-line messages
                || message.contains("Play as ") // Setup round civilization messages
        }

        // Handle log entries before the first player turn (age 0)
        let first_player_log_index = player_log_ranges
            .first()
            .map(|(_, _, _, log_index)| *log_index)
            .unwrap_or(rc.game.log.len());

        for log_index in 0..first_player_log_index {
            if let Some(log_entries_for_turn) = rc.game.log.get(log_index) {
                for message in log_entries_for_turn {
                    // Simulate multiline labels to get accurate line count
                    multiline_label(
                        rc.state,
                        message,
                        Self::max_width(rc),
                        |label: &str| {
                            log_entries.push(LogEntry {
                                age: 0,
                                round: 0,
                                player_name: if should_skip_player_name(label) {
                                    String::new()
                                } else {
                                    "Setup".to_string()
                                },
                                message: label.to_string(),
                            });
                        },
                    );
                }
            }
        }

        // Process each player's log range
        for i in 0..player_log_ranges.len() {
            let (age, round, player_index, log_index_before_turn) = player_log_ranges[i];

            // The log_index from server represents the last log entry BEFORE this player's turn starts
            // So the actual start of this player's entries is log_index + 1
            let start_log_index = log_index_before_turn + 1;

            // Find the end index - this should be the start of the NEXT player's turn
            let end_log_index = if i + 1 < player_log_ranges.len() {
                // The next player's log_index is also the last entry before their turn
                // So their turn starts at next_log_index + 1
                // Which means current player's entries end at next_log_index + 1 (exclusive)
                let next_log_index_before_turn = player_log_ranges[i + 1].3;
                next_log_index_before_turn + 1
            } else {
                // This is the last player, so include everything to the end
                rc.game.log.len()
            };

            let player_name = rc.game.player(player_index).get_name();

            // Get all log entries for this player's range
            // Note: The range is [start_log_index, end_log_index) - exclusive end
            for log_index in start_log_index..end_log_index {
                if let Some(log_entries_for_turn) = rc.game.log.get(log_index) {
                    for message in log_entries_for_turn {
                        // Simulate multiline labels to get accurate line count
                        multiline_label(
                            rc.state,
                            message,
                            Self::max_width(rc),
                            |label: &str| {
                                log_entries.push(LogEntry {
                                    age,
                                    round,
                                    player_name: if should_skip_player_name(label) {
                                        String::new()
                                    } else {
                                        player_name.clone()
                                    },
                                    message: label.to_string(),
                                });
                            },
                        );
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

    for entry in &d.log_entries[start..end] {
        let age_text = entry.age.to_string();
        let round_text = entry.round.to_string();
        let player_text = &entry.player_name;
        let message_text = &entry.message;

        // Calculate positions for each column
        let age_pos = vec2(30., y * 25. + 20.);
        let round_pos = vec2(80., y * 25. + 20.);
        let player_pos = vec2(140., y * 25. + 20.);
        let message_pos = vec2(280., y * 25. + 20.);

        // Draw each column
        rc.draw_text(&age_text, age_pos.x, age_pos.y);
        rc.draw_text(&round_text, round_pos.x, round_pos.y);
        rc.draw_text(player_text, player_pos.x, player_pos.y);
        rc.draw_text(message_text, message_pos.x, message_pos.y);

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
