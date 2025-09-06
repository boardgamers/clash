use crate::log_ui::{LogBody, LogEntry, multiline_label};
use crate::render_context::RenderContext;
use server::action::Action;
use server::events::EventOrigin;
use server::game_setup::setup_event_origin;
use server::log::{ActionLogAge, TurnType};
use server::status_phase::StatusPhaseStateType;

struct LogCollector {
    log_entries: Vec<LogEntry>,
    active_origin: Option<EventOrigin>,
}

impl LogCollector {
    fn new() -> Self {
        Self {
            log_entries: Vec::new(),
            active_origin: None,
        }
    }

    fn add_log_entry(&mut self, body: LogBody, indent: usize) {
        self.log_entries
            .push(LogEntry::new(body, self.active_origin.clone(), indent));
    }

    fn add_message(&mut self, message: &str, indent: usize) {
        self.add_log_entry(LogBody::Message(message.to_string()), indent);
    }
}

pub(crate) fn collect_log_entries(rc: &RenderContext) -> Vec<LogEntry> {
    let mut c = LogCollector::new();

    for age in &rc.game.log {
        if age.age == 0 {
            c.add_message("Game Start", 0);
        }
        for round in &age.rounds {
            if round.round > 0 {
                c.add_message(&format!("Age {}, Round {}", age.age, round.round), 0);
            }
            for turn in &round.turns {
                c.active_origin = None;
                let indent = match turn.turn_type {
                    TurnType::Player(p) => {
                        c.add_log_entry(
                            LogBody::PlayerTurn {
                                age: age.age,
                                round: round.round,
                                player: p,
                            },
                            1,
                        );
                        2
                    }
                    TurnType::Setup(p) => {
                        c.active_origin = Some(setup_event_origin());
                        c.add_log_entry(
                            LogBody::PlayerSetup {
                                player: p,
                                civilization: rc.game.player(p).civilization.name.clone(),
                            },
                            1,
                        );
                        2
                    }
                    TurnType::StatusPhase(t) => status_phase_title(age, t, &mut c),
                };

                for action in &turn.actions {
                    let origin = &action.origin;
                    if origin.is_some() {
                        c.active_origin.clone_from(origin);
                    }
                    let indent = if matches!(action.action, Action::StartTurn) {
                        indent
                    } else {
                        c.add_log_entry(LogBody::Action(action.clone()), indent);
                        indent + 1
                    };
                    for item in &action.items {
                        c.add_log_entry(LogBody::Item(item.clone()), indent);
                    }
                    for message in &action.log {
                        // Simulate multiline labels to get accurate line count
                        multiline_label(rc.state, message, max_width(rc), |label: &str| {
                            c.add_message(label, indent);
                        });
                    }
                }
            }
        }
    }
    c.log_entries
}

fn status_phase_title(age: &ActionLogAge, t: StatusPhaseStateType, c: &mut LogCollector) -> usize {
    if matches!(t, StatusPhaseStateType::CompleteObjectives) {
        c.add_message(&format!("Status Phase (Age {})", age.age), 2);
    }
    c.add_message(
        match t {
            StatusPhaseStateType::CompleteObjectives => "Complete Objectives",
            StatusPhaseStateType::FreeAdvance => "Free Advance",
            StatusPhaseStateType::DrawCards => "Draw Cards",
            StatusPhaseStateType::RazeSize1City => "Raze Size 1 City",
            StatusPhaseStateType::ChangeGovernmentType => "Change Government Type",
            StatusPhaseStateType::DetermineFirstPlayer => "Determine First Player",
        },
        3,
    );
    4
}

fn max_width(rc: &RenderContext) -> f32 {
    rc.state.screen_size.x - 300.
}
