use crate::city_ui::draw_mood_state;
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, State, StateUpdate};
use crate::layout_ui::{FONT_SIZE, bottom_center_texture, draw_scaled_icon};
use crate::log_collector::collect_log_entries;
use crate::render_context::RenderContext;
use macroquad::math::{Vec2, vec2};
use macroquad::prelude::{BLACK, Color, Texture2D};
use server::action::Action;
use server::card::hand_card_message;
use server::content::persistent_events::EventResponse;
use server::events::EventOrigin;
use server::log::{
    ActionLogAction, ActionLogBalance, ActionLogEntry, ActionLogEntryMove, ActionLogEntryStructure,
    ActionLogIncidentToken, ActionLogItem,
};
use server::movement::MovementAction;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::unit::Units;

#[derive(Clone, Debug)]
pub(crate) enum LogBody {
    Message(String),
    Item(ActionLogItem),
    PlayerSetup { player: usize, civilization: String },
    PlayerTurn { age: u32, round: u32, player: usize },
    Action(ActionLogAction),
}

#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    body: LogBody,
    active_origin: Option<EventOrigin>,
    indent: usize,
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
    let message_pos = vec2(20. + entry.indent as f32 * 20.0, y * 25. + 20.);

    let mut drawer = RichTextDrawer::new(rc, message_pos);

    match &entry.body {
        LogBody::Message(m) => rc.draw_text(m, message_pos.x, message_pos.y),
        LogBody::Item(item) => {
            drawer.player(item.player);
            let balance = item.entry.balance();
            if let Some(b) = &balance {
                drawer.text(match b {
                    ActionLogBalance::Gain => "gains",
                    ActionLogBalance::Loss => "loses",
                    ActionLogBalance::Pay => "pays",
                });
            }

            draw_item(&mut drawer, item);

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
        LogBody::PlayerSetup {
            player,
            civilization,
        } => {
            drawer.player(*player);
            drawer.text(&format!("plays as {civilization}"));
        }
        LogBody::PlayerTurn { age, round, player } => {
            drawer.player(*player);
            drawer.text(&format!("starts their turn (Age {age}, Round {round})"));
        }
        LogBody::Action(a) => {
            drawer.player(a.player);
            draw_action_log_entry(&mut drawer, a);
        }
    }
}

struct RichTextDrawer<'a> {
    rc: &'a RenderContext<'a>,
    current_pos: Vec2,
    space: f32,
}

impl RichTextDrawer<'_> {
    fn new<'a>(rc: &'a RenderContext<'_>, start_pos: Vec2) -> RichTextDrawer<'a> {
        RichTextDrawer {
            rc,
            current_pos: start_pos,
            space: rc.state.measure_text(" ").width,
        }
    }

    fn text(&mut self, text: &str) {
        self.text_ex(text, BLACK, FONT_SIZE);
    }

    fn text_ex(&mut self, text: &str, color: Color, font_size: u16) {
        self.rc.draw_text_ex(
            text,
            self.current_pos.x,
            self.current_pos.y,
            color,
            font_size,
        );
        self.current_pos.x += self.rc.state.measure_text(text).width + self.space;
    }

    fn modifier(&mut self, modifier: &str, balance: Option<&ActionLogBalance>) {
        let verb = match balance {
            None | Some(ActionLogBalance::Gain) => "using",
            Some(ActionLogBalance::Loss) => "to",
            Some(ActionLogBalance::Pay) => "for",
        };
        self.text(&format!("{verb} {modifier}"));
    }

    fn icon(&mut self, texture: &Texture2D) {
        self.icon_with_size(texture, 20.0);
    }

    fn icon_with_size(&mut self, texture: &Texture2D, size: f32) {
        draw_scaled_icon(
            &self.rc.no_icon_background(),
            texture,
            "",
            vec2(self.current_pos.x, self.current_pos.y - size / 2.0 - 5.0),
            size,
        );
        self.current_pos.x += size + self.space;
    }

    fn player(&mut self, player: usize) {
        self.text_ex(
            &self.rc.game.player_name(player),
            self.rc.player_color(player),
            FONT_SIZE,
        );
        self.current_pos.x += self.space;
    }

    fn at_location(&mut self, position: Position) {
        self.text("at");
        self.location(position);
    }

    fn location(&mut self, position: Position) {
        self.icon_with_size(&self.rc.assets().hex, 35.0);
        self.current_pos.x -= 31.0;
        self.text_ex(&format!("{position}"), BLACK, 17);
    }

    fn resources(&mut self, resources: &ResourcePile) {
        for (resource, amount) in resources.clone() {
            if amount > 0
                && let Some(texture) = self.rc.assets().resources.get(&resource)
            {
                self.icon(texture);
                self.text(&amount.to_string());
            }
        }
    }

    fn units(&mut self, units: &Units) {
        for (unit, amount) in units.clone() {
            if amount > 0 {
                let texture = self.rc.assets().unit(unit, self.rc.shown_player);
                self.icon(texture);
                let mut u = Units::empty();
                for _ in 0..amount {
                    u += &unit;
                }
                self.text(&u.to_string(Some(self.rc.game)));
            }
        }
    }
}

fn draw_item(drawer: &mut RichTextDrawer, item: &ActionLogItem) {
    match &item.entry {
        ActionLogEntry::Action { balance: _, amount } => {
            if let Some(a) = amount {
                drawer.text(&a.to_string());
            }
            drawer.icon(&drawer.rc.assets().end_turn);
        }
        ActionLogEntry::Resources {
            resources,
            balance: _,
        } => {
            drawer.resources(resources);
        }
        ActionLogEntry::Advance {
            advance,
            balance: _,
            incident_token,
        } => {
            drawer.icon(&drawer.rc.assets().advances);
            drawer.text(advance.name(drawer.rc.game));
            if let ActionLogIncidentToken::Take(t) = incident_token {
                if *t > 0 {
                    drawer.text(&format!("and take an event token ({t} left)"));
                } else {
                    drawer.text("and take an event token (triggering an incident)");
                }
            }
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
            draw_structure(drawer, item, s);
        }
        ActionLogEntry::HandCard { card, from, to } => {
            let (_, message) = hand_card_message(drawer.rc.game, card, from, to);
            drawer.text(&message);
        }
        ActionLogEntry::MoodChange { city, mood } => {
            drawer.text("City");
            drawer.at_location(*city);
            drawer.text(&format!("becomes {mood}"));
            let c = center(drawer.current_pos);
            drawer
                .rc
                .draw_circle(c, RADIUS, drawer.rc.player_color(item.player));
            draw_mood_state(drawer.rc, c, mood);
            drawer.current_pos.x += 35.0;
        }
        ActionLogEntry::Move(m) => draw_move(drawer, m),
        ActionLogEntry::Explore { tiles } => {
            drawer.text("explores:");
            for (p, t) in tiles {
                drawer.location(*p);
                drawer.text(&format!("is {t}"));
            }
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
            ("sailed", "")
        } else {
            ("disembarked", "")
        }
    } else if m.embark_carrier_id.is_some() {
        ("embarked", "")
    } else if m.start.is_neighbor(m.destination) {
        ("marched", "")
    } else {
        ("marched", " on roads")
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

fn draw_structure(drawer: &mut RichTextDrawer, item: &ActionLogItem, s: &ActionLogEntryStructure) {
    // Draw structure icon or circle
    let c = center(drawer.current_pos);
    let r = RADIUS;
    match &s.structure {
        Structure::CityCenter => {
            drawer
                .rc
                .draw_circle(c, r, drawer.rc.player_color(item.player));
            drawer.current_pos.x += 35.0;
            drawer.text("City");
        }
        Structure::Building(b) => {
            draw_scaled_icon(drawer.rc, &drawer.rc.assets().buildings[b], "", c, r);
            drawer.current_pos.x += 35.0;
            drawer.text(b.name());
        }
        Structure::Wonder(w) => {
            draw_scaled_icon(drawer.rc, &drawer.rc.assets().wonders[w], "", c, r);
            drawer.current_pos.x += 35.0;
            drawer.text(&w.name());
        }
    }
    drawer.at_location(s.position);
    if let Some(port_pos) = s.port_position {
        drawer.text(" at the water tile");
        drawer.at_location(port_pos);
    }
}

const RADIUS: f32 = 15.0;

fn center(message_pos: Vec2) -> Vec2 {
    vec2(message_pos.x + 15.0, message_pos.y - RADIUS / 2.0)
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

fn draw_action_log_entry(drawer: &mut RichTextDrawer, a: &ActionLogAction) {
    match &a.action {
        Action::Playing(p) => draw_playing_action(drawer, p, a),
        Action::Movement(m) => match m {
            MovementAction::Move(_) => {
                drawer.text("performs a move action");
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
        EventResponse::SelectPlayer(p) => drawer.text(&format!(
            "selects player: {}",
            drawer.rc.game.player_name(*p)
        )),
        EventResponse::SelectPositions(p) => drawer.text(&format!("selects positions: {p:?}")),
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

fn draw_playing_action(drawer: &mut RichTextDrawer, p: &PlayingAction, a: &ActionLogAction) {
    match p {
        PlayingAction::Advance(_) => {
            drawer.text("advances");
        }
        PlayingAction::FoundCity { .. } => {
            drawer.text("founds a city");
        }
        PlayingAction::Construct(_) => {
            drawer.text("builds");
        }
        PlayingAction::Collect(c) => {
            drawer.text("collects");
            drawer.at_location(c.city_position);
        }
        PlayingAction::Recruit(_) => {
            drawer.text("recruits");
        }
        PlayingAction::IncreaseHappiness(_) => {
            drawer.text("increases happiness");
        }
        PlayingAction::InfluenceCultureAttempt(_) => {
            drawer.text("attempts cultural influence");
        }
        PlayingAction::Custom(c) => {
            drawer.text(&format!(
                "starts {}",
                a.items[0].origin.name(drawer.rc.game)
            ));
            if let Some(pos) = c.city {
                drawer.at_location(pos);
            }
        }
        PlayingAction::ActionCard(a) => {
            drawer.text(&format!(
                "plays Action Card: {}",
                drawer.rc.game.cache.get_action_card(*a).name()
            ));
        }
        PlayingAction::WonderCard(w) => {
            drawer.text(&format!("plays Wonder Card: {}", w.name()));
        }
        PlayingAction::EndTurn => {
            drawer.text("ends their turn");
        }
    }
}
