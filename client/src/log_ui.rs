use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, State, StateUpdate};
use crate::layout_ui::bottom_center_texture;
use crate::log_collector::{ActionLogBody, LogBody, LogEntry, collect_log_entries};
use crate::render_context::RenderContext;
use crate::richtext::RichTextDrawer;
use macroquad::math::vec2;
use server::action::Action;
use server::card::hand_card_message;
use server::combat_roll::UnitCombatRoll;
use server::content::custom_actions::SpecialAction;
use server::content::persistent_events::EventResponse;
use server::events::EventOrigin;
use server::log::{
    ActionLogBalance, ActionLogEntry, ActionLogEntryAdvance, ActionLogEntryMove,
    ActionLogIncidentToken, ActionLogItem,
};
use server::movement::MovementAction;
use server::playing_actions::{PlayingAction, PlayingActionType};
use server::unit::UnitType;

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

        let log_entries = collect_log_entries(rc);

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
}

pub(crate) fn show_log(rc: &RenderContext, d: &LogDialog) -> RenderResult {
    let state = rc.state;

    // Use the pre-calculated log entries
    let start = d.current_page * d.lines_per_page;
    let end = usize::min(start + d.lines_per_page, d.log_entries.len());
    let mut y = 1.5;

    for entry in &d.log_entries[start..end] {
        draw_line(rc, y, entry);
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

fn draw_line(rc: &RenderContext, y: f32, entry: &LogEntry) {
    let message_pos = vec2(20. + entry.indent as f32 * 30.0, y * 25. + 20.);

    let mut drawer = RichTextDrawer::new(rc, message_pos);

    match &entry.body {
        LogBody::Message(m) => rc.draw_text(m, message_pos.x, message_pos.y),
        LogBody::Item(item) => {
            draw_item(entry, &mut drawer, item);
        }
        LogBody::PlayerSetup(t) => {
            drawer.player(t.player);
            if let Some(civilization) = &t.civilization {
                drawer.text("plays as");
                drawer.text(civilization);
            } else {
                drawer.text("starts without a civilization");
            }
        }
        LogBody::PlayerTurn { age, round, player } => {
            drawer.player(*player);
            drawer.text(&format!("starts their turn (Age {age}, Round {round})"));
        }
        LogBody::Action(a) => {
            draw_action(&mut drawer, a);
        }
        LogBody::CombatUnits {
            role,
            player,
            units,
        } => {
            drawer.text(role);
            drawer.player(*player);
            drawer.text("with");
            drawer.units(units);
        }
        LogBody::DieRoll(r) => {
            draw_die_roll(&mut drawer, r);
        }
        LogBody::InfluenceStartCity(i) => {
            drawer.player(i.player);
            drawer.text("uses start city");
            drawer.at_location(i.info.starting_city_position);
        }
        LogBody::InfluenceRangeBoost(i) => {
            drawer.player(i.player);
            drawer.text("may pay");
            drawer.resources(&i.info.range_boost_cost.default);
            drawer.text("to boost the range");
        }
    }
}

fn draw_item(entry: &LogEntry, drawer: &mut RichTextDrawer, item: &ActionLogItem) {
    drawer.player(item.player);
    let balance = item.entry.balance();
    if let Some(b) = &balance {
        drawer.text(match b {
            ActionLogBalance::Gain => "gains",
            ActionLogBalance::Loss => "loses",
            ActionLogBalance::Pay => "pays",
        });
    }

    draw_entry(drawer, &item.entry, item.player);

    if entry
        .active_origin
        .as_ref()
        .is_none_or(|o| o != &item.origin)
    {
        drawer.modifier(&item.origin.name(drawer.rc.game), balance);
    }
    item.modifiers.iter().for_each(|m| {
        drawer.modifier(&m.name(drawer.rc.game), balance);
    });
}

fn draw_die_roll(drawer: &mut RichTextDrawer, r: &UnitCombatRoll) {
    let (bonus_str, add_value) = if r.bonus {
        match r.unit_type {
            UnitType::Settler | UnitType::Ship => panic!("unit does not have bonus"),
            UnitType::Infantry => (Some("+1 combat value"), true),
            UnitType::Cavalry => (Some("+2 combat value"), true),
            UnitType::Elephant => (Some("-1 hits, no combat value"), false),
            UnitType::Leader(_) => (Some("re-roll"), false),
        }
    } else {
        (None, true)
    };

    if add_value {
        drawer.text(&format!("rolls a {}", r.value));
    } else {
        drawer.text(&format!("rolls a ({})", r.value));
    }
    drawer.unit_icon(r.unit_type);
    if let Some(bonus_str) = bonus_str {
        drawer.text(bonus_str);
    }
}

fn draw_entry(drawer: &mut RichTextDrawer, entry: &ActionLogEntry, player: usize) {
    match entry {
        ActionLogEntry::Action { balance: _, amount } => {
            if let Some(a) = amount {
                drawer.text(&a.to_string());
            }
            drawer.action_icon();
        }
        ActionLogEntry::Resources {
            resources,
            balance: _,
        } => {
            drawer.resources(resources);
        }
        ActionLogEntry::Advance(a) => {
            draw_advance(drawer, a);
        }
        ActionLogEntry::Units {
            units,
            balance: _,
            position,
        } => {
            drawer.units(units);
            drawer.at_location(*position);
        }
        ActionLogEntry::Structure(s) => {
            drawer.structure(&s.structure, s.position, s.port_position, player);
        }
        ActionLogEntry::HandCard { card, from, to } => {
            let (_, message) = hand_card_message(drawer.rc.game, card, from, to);
            drawer.text(&message);
        }
        ActionLogEntry::MoodChange { city, mood } => {
            drawer.text("City");
            drawer.at_location(*city);
            drawer.text(&format!("becomes {mood}"));
            drawer.mood(player, mood);
        }
        ActionLogEntry::Move(m) => draw_move(drawer, m),
        ActionLogEntry::Explore { tiles } => {
            drawer.text("explores:");
            for (p, t) in tiles {
                drawer.location(*p);
                drawer.text(&format!("is {t}"));
            }
        }
        ActionLogEntry::CombatRound(r) => {
            drawer.text(&format!("Combat Round {}", r.round));
        }
        ActionLogEntry::CombatRoll(r) => {
            drawer.text(&format!(
                "rolls for combined combat value of {} and gets {} hits",
                r.combat_value, r.hits,
            ));
        }
        ActionLogEntry::Text(m) => {
            drawer.text(m);
        }
        ActionLogEntry::InfluenceCultureAttempt(i) => {
            drawer.structure(&i.structure, i.position, None, i.target_player);
        }
    }
}

fn draw_advance(drawer: &mut RichTextDrawer, advance: &ActionLogEntryAdvance) {
    drawer.icon(&drawer.rc.assets().advances);
    drawer.text(advance.advance.name(drawer.rc.game));
    if let ActionLogIncidentToken::Take(t) = advance.incident_token {
        if t > 0 {
            drawer.text(&format!("and takes an event token ({t} left)"));
        } else {
            drawer.text("and takes the last event token - triggering an event!");
        }
    }
}

fn draw_move(drawer: &mut RichTextDrawer, m: &ActionLogEntryMove) {
    let game = drawer.rc.game;
    let t = game
        .map
        .get(m.destination)
        .expect("the destination position should be on the map");
    let (verb, suffix) = if game.map.is_sea(m.start) {
        if t.is_unexplored() || t.is_water() {
            ("sails", "")
        } else {
            ("disembarks", "")
        }
    } else if m.embark_carrier_id.is_some() {
        ("embarks", "")
    } else if m.start.is_neighbor(m.destination) {
        ("marches", "")
    } else {
        ("marches", " on roads")
    };
    drawer.text(verb);
    drawer.units(&m.units);
    drawer.text("from");
    drawer.location(m.start);
    drawer.text("to");
    drawer.location(m.destination);
    if !suffix.is_empty() {
        drawer.text(suffix);
    }
}

pub(crate) fn multiline_label(
    state: &State,
    label: &str,
    len: f32,
    mut print: impl FnMut(usize, &str),
) {
    let mut line = String::new();
    let mut i = 0;
    label.split(' ').for_each(|s| {
        let next = format!("{line} {s}");
        let dimensions = state.measure_text(&next);
        if dimensions.width > len {
            print(i, &line);
            i += 1;
            line = "    ".to_string();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(s);
    });
    if !line.is_empty() {
        print(i, &line);
    }
}

fn draw_action(drawer: &mut RichTextDrawer, body: &ActionLogBody) {
    if body.action_cost {
        drawer.action_icon();
    } else {
        drawer.icon(&drawer.rc.assets().fast);
    }
    let a = &body.action;
    drawer.player(a.player);

    match &a.action {
        Action::Playing(p) => draw_playing_action(drawer, p, body),
        Action::Movement(m) => match m {
            MovementAction::Move(_) => {
                draw_argument(drawer, body);
            }
            MovementAction::Stop => {
                drawer.text("stops movement");
            }
        },
        Action::Response(r) => draw_response_action(drawer, r),
        Action::Undo => panic!("Unexpected undo in log"),
        Action::Redo => panic!("Unexpected redo in log"),
        Action::StartTurn => panic!("Unexpected start turn in log"),
        Action::ChooseCivilization(c) => {
            drawer.text(&format!("Choose Civilization: {c}"));
        }
    }
}

fn draw_response_action(drawer: &mut RichTextDrawer, r: &EventResponse) {
    match r {
        EventResponse::SelectAdvance(_) => drawer.text("selects advance"),
        EventResponse::Payment(_) => drawer.text("selects payment"),
        EventResponse::ResourceReward(_) => drawer.text("receives"),
        EventResponse::SelectPlayer(_p) => drawer.text("selects a player"),
        EventResponse::SelectPositions(p) => {
            drawer.text("selects positions");
            for pos in p {
                drawer.location(*pos);
            }
        }
        EventResponse::SelectUnitType(u) => {
            drawer.text(&format!("select unit type: {}", u.name(drawer.rc.game)));
        }
        EventResponse::SelectUnits(_u) => drawer.text("selects units"),
        EventResponse::SelectHandCards(_h) => drawer.text("selects hand cards"),
        EventResponse::SelectStructures(_s) => drawer.text("select structures"),
        EventResponse::Bool(b) => {
            if *b {
                drawer.text("accepts");
            } else {
                drawer.text("declines");
            }
        }
        EventResponse::ChangeGovernmentType(c) => {
            drawer.text(&format!("changes government to {}", c.new_government));
        }
        EventResponse::ExploreResolution(_) => drawer.text("chooses rotation"),
    }
}

fn draw_playing_action(drawer: &mut RichTextDrawer, p: &PlayingAction, body: &ActionLogBody) {
    let a = &body.action;
    let origin = body.action.origin.as_ref();
    match p {
        PlayingAction::Advance(_) => {
            drawer.text("advances");
            draw_argument(drawer, body);
        }
        PlayingAction::FoundCity { .. } => {
            drawer.text("founds a");
            draw_argument(drawer, body);
        }
        PlayingAction::Construct(_) => {
            drawer.text("builds a");
            draw_argument(drawer, body);
        }
        PlayingAction::Collect(c) => {
            drawer.text("collects");
            drawer.at_location(c.city_position);
            draw_modifier_suffix(drawer, &c.action_type, origin);
        }
        PlayingAction::Recruit(_) => {
            drawer.text("recruits");
            draw_argument(drawer, body);
        }
        PlayingAction::IncreaseHappiness(i) => {
            drawer.text("increases happiness");
            draw_modifier_suffix(drawer, &i.action_type, origin);
        }
        PlayingAction::InfluenceCultureAttempt(a) => {
            drawer.text("attempts to influence");
            draw_argument(drawer, body);
            draw_modifier_suffix(drawer, &a.action_type, origin);
        }
        PlayingAction::Custom(c) => {
            drawer.text(&format!(
                "starts {}",
                a.origin
                    .as_ref()
                    .expect("origin should be set for custom actions")
                    .name(drawer.rc.game)
            ));
            if let Some(pos) = c.city {
                drawer.at_location(pos);
            }
        }
        PlayingAction::ActionCard(a) => {
            drawer.text(&format!(
                "plays action card: {}",
                drawer.rc.game.cache.get_action_card(*a).name()
            ));
        }
        PlayingAction::WonderCard(w) => {
            drawer.text("plays wonder card");
            drawer.wonder(*w);
        }
        PlayingAction::EndTurn => {
            drawer.text("ends their turn");
        }
    }
}

fn draw_modifier_suffix(
    drawer: &mut RichTextDrawer,
    action_type: &PlayingActionType,
    origin: Option<&EventOrigin>,
) {
    if let PlayingActionType::Special(SpecialAction::Modifier(_)) = action_type {
        drawer.text(&format!(
            "using {}",
            origin
                .as_ref()
                .expect("origin not found")
                .name(drawer.rc.game)
        ));
    }
}

fn draw_argument(drawer: &mut RichTextDrawer, body: &ActionLogBody) {
    if let Some(a) = &body.argument {
        draw_entry(drawer, a, body.action.player);
    } else {
        panic!("subject missing advance details");
    }
}
