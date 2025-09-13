use crate::log_ui::multiline_label;
use crate::render_context::RenderContext;
use server::action::Action;
use server::card::HandCardLocation;
use server::combat_roll::UnitCombatRoll;
use server::content::ability::combat_event_origin;
use server::cultural_influence::InfluenceCultureAttemptInfo;
use server::events::EventOrigin;
use server::game_setup::setup_event_origin;
use server::log::{
    ActionLogAction, ActionLogAge, ActionLogBalance, ActionLogEntry, ActionLogItem, ActionLogRound,
    ActionLogTurn, SetupTurnType, TurnType,
};
use server::playing_actions::PlayingAction;
use server::status_phase::StatusPhaseStateType;
use server::unit::Units;
use server::utils::remove_element_by;

#[derive(Clone, Debug)]
pub(crate) struct ActionLogInfluence {
    pub(crate) player: usize,
    pub(crate) info: InfluenceCultureAttemptInfo,
}

#[derive(Clone, Debug)]
pub(crate) struct ActionLogBody {
    pub(crate) action: ActionLogAction,
    pub(crate) action_cost: bool,
    pub(crate) argument: Option<ActionLogEntry>,
}

impl ActionLogBody {
    pub(crate) fn new(action: ActionLogAction) -> Self {
        ActionLogBody {
            action,
            action_cost: false,
            argument: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum LogBody {
    Message(String),
    Item(ActionLogItem),
    PlayerSetup(SetupTurnType),
    PlayerTurn {
        age: u32,
        round: u32,
        player: usize,
    },
    Action(ActionLogBody),
    CombatUnits {
        role: String,
        player: usize,
        units: Units,
    },
    DieRoll(UnitCombatRoll),
    InfluenceStartCity(ActionLogInfluence),
    InfluenceRangeBoost(ActionLogInfluence),
}

#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    pub(crate) body: LogBody,
    pub(crate) active_origin: Option<EventOrigin>,
    pub(crate) indent: usize,
}

impl LogEntry {
    pub(crate) fn new(body: LogBody, active_origin: Option<EventOrigin>, indent: usize) -> Self {
        LogEntry {
            body,
            active_origin,
            indent,
        }
    }
}

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

    fn add_entry(&mut self, body: LogBody, indent: usize) {
        self.log_entries
            .push(LogEntry::new(body, self.active_origin.clone(), indent));
    }

    fn add_message(&mut self, message: &str, indent: usize) {
        self.add_entry(LogBody::Message(message.to_string()), indent);
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
                let indent = add_title_items(&mut c, age, round, turn);
                for action in &turn.actions {
                    collect_action(rc, &mut c, action, indent + action.active_events);
                }
            }
        }
    }
    c.log_entries
}

fn collect_action(
    rc: &RenderContext,
    c: &mut LogCollector,
    action: &ActionLogAction,
    indent: usize,
) {
    let origin = &action.origin;
    if origin.is_some() {
        c.active_origin.clone_from(origin);
    }
    let mut items = action.items.clone();
    let indent = if matches!(action.action, Action::StartTurn) {
        indent
    } else {
        let mut body = ActionLogBody::new(action.clone());
        inline_action_items(&mut items, &mut body);
        c.add_entry(LogBody::Action(body.clone()), indent);
        add_additional_action_items(c, &mut body, action, indent + 1);
        indent + 1
    };

    for item in &items {
        match &item.entry {
            ActionLogEntry::Text(m) => {
                add_text_item(rc, c, indent, item, m);
                continue;
            }
            ActionLogEntry::HandCard {
                to: HandCardLocation::PlayToKeep,
                ..
            } => {
                // redundant
                continue;
            }
            _ => {}
        }

        set_item_origin(c, item);
        c.add_entry(LogBody::Item(item.clone()), indent);
        add_additional_items(c, item, indent);
    }
}

fn add_additional_action_items(
    c: &mut LogCollector,
    body: &mut ActionLogBody,
    action: &ActionLogAction,
    indent: usize,
) {
    if let Some(ActionLogEntry::InfluenceCultureAttempt(i)) = &body.argument {
        c.add_entry(
            LogBody::InfluenceStartCity(ActionLogInfluence {
                player: action.player,
                info: i.clone(),
            }),
            indent,
        );
        if !i.range_boost_cost.is_free() {
            c.add_entry(
                LogBody::InfluenceRangeBoost(ActionLogInfluence {
                    player: action.player,
                    info: i.clone(),
                }),
                indent,
            );
        }
    }
}

fn add_text_item(
    rc: &RenderContext,
    c: &mut LogCollector,
    indent: usize,
    item: &ActionLogItem,
    m: &str,
) {
    // Simulate multiline labels to get accurate line count
    multiline_label(rc.state, m, max_width(rc), |i, label: &str| {
        if i == 0 {
            c.add_entry(LogBody::Item(item.clone()), indent);
        } else {
            c.add_message(label, indent);
        }
    });
}

fn set_item_origin(c: &mut LogCollector, item: &ActionLogItem) {
    if matches!(
        item.entry,
        ActionLogEntry::CombatRound(_) | ActionLogEntry::CombatRoll(_)
    ) {
        c.active_origin = Some(combat_event_origin());
    }
}

fn add_title_items(
    c: &mut LogCollector,
    age: &ActionLogAge,
    round: &ActionLogRound,
    turn: &ActionLogTurn,
) -> usize {
    match &turn.turn_type {
        TurnType::Player(p) => {
            c.add_entry(
                LogBody::PlayerTurn {
                    age: age.age,
                    round: round.round,
                    player: *p,
                },
                1,
            );
            2
        }
        TurnType::Setup(t) => {
            c.active_origin = Some(setup_event_origin());
            c.add_entry(LogBody::PlayerSetup(t.clone()), 1);
            2
        }
        TurnType::StatusPhase(t) => status_phase_title(age, *t, c),
    }
}

fn add_additional_items(c: &mut LogCollector, item: &ActionLogItem, indent: usize) {
    match &item.entry {
        ActionLogEntry::CombatRound(r) => {
            c.add_entry(
                LogBody::CombatUnits {
                    role: "Attacking:".to_string(),
                    player: item.player,
                    units: r.attackers.clone(),
                },
                indent + 1,
            );
            c.add_entry(
                LogBody::CombatUnits {
                    role: "Defending:".to_string(),
                    player: r.defending_player,
                    units: r.defenders.clone(),
                },
                indent + 1,
            );
        }
        ActionLogEntry::CombatRoll(r) => {
            for m in &r.combat_modifiers {
                c.add_message(m, indent + 1);
            }
            for r in &r.rolls {
                c.add_entry(LogBody::DieRoll(r.clone()), indent + 1);
            }
        }
        _ => {}
    }
}

fn inline_action_items(items: &mut Vec<ActionLogItem>, action: &mut ActionLogBody) {
    if remove_element_by(items, |item| {
        matches!(
            item,
            ActionLogItem {
                player,
                entry: ActionLogEntry::Action {
                    balance: ActionLogBalance::Pay,
                    ..
                },
                ..
            } if *player == action.action.player
        )
    })
    .is_some()
    {
        action.action_cost = true;
    }
    action.argument = pull_action_arg(items, action);
}

pub fn find_action_arg(
    list: &mut Vec<ActionLogItem>,
    body: &ActionLogBody,
    f: impl Fn(&ActionLogItem) -> bool,
) -> Option<ActionLogEntry> {
    let arg = remove_element_by(list, |i| f(i) && i.player == body.action.player).map(|i| i.entry);
    assert!(arg.is_some(), "Could not find action argument in log items");
    arg
}

fn pull_action_arg(items: &mut Vec<ActionLogItem>, body: &ActionLogBody) -> Option<ActionLogEntry> {
    match &body.action.action {
        Action::Playing(a) => match a {
            PlayingAction::Advance(_) => find_action_arg(items, body, |item| {
                matches!(
                    item,
                    ActionLogItem {
                        entry: ActionLogEntry::Advance(_),
                        ..
                    }
                )
            }),
            PlayingAction::Collect(_) => find_action_arg(items, body, |item| {
                matches!(
                    item,
                    ActionLogItem {
                        entry: ActionLogEntry::Resources {
                            balance: ActionLogBalance::Gain,
                            ..
                        },
                        ..
                    }
                )
            }),
            PlayingAction::FoundCity { .. } | PlayingAction::Construct(_) => {
                find_action_arg(items, body, |item| {
                    matches!(
                        item,
                        ActionLogItem {
                            entry: ActionLogEntry::Structure(_),
                            ..
                        }
                    )
                })
            }
            PlayingAction::Recruit(_) => find_action_arg(items, body, |item| {
                matches!(
                    item,
                    ActionLogItem {
                        entry: ActionLogEntry::Units { .. },
                        ..
                    }
                )
            }),
            PlayingAction::InfluenceCultureAttempt(_) => find_action_arg(items, body, |item| {
                matches!(
                    item,
                    ActionLogItem {
                        entry: ActionLogEntry::InfluenceCultureAttempt(_),
                        ..
                    }
                )
            }),
            PlayingAction::ActionCard(_) => find_action_arg(items, body, |item| {
                matches!(
                    item,
                    ActionLogItem {
                        entry: ActionLogEntry::HandCard { .. },
                        ..
                    }
                )
            }),
            _ => None,
        },
        Action::Movement(_) => find_action_arg(items, body, |item| {
            matches!(
                item,
                ActionLogItem {
                    entry: ActionLogEntry::Move(_),
                    ..
                }
            )
        }),
        _ => None,
    }
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
    if matches!(t, StatusPhaseStateType::DrawCards) {
        4
    } else {
        3 // response adds another indent
    }
}

fn max_width(rc: &RenderContext) -> f32 {
    rc.state.screen_size.x - 300.
}
