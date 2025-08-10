use crate::action_buttons::action_buttons;
use crate::city_ui::city_labels;
use crate::client::Features;
use crate::client_state::{NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::{OkTooltip, ok_button};
use crate::layout_ui::{
    ICON_SIZE, UI_BACKGROUND, bottom_center_anchor, bottom_center_texture, bottom_centered_text,
    bottom_right_texture, button_pressed, icon_pos, top_center_anchor, top_center_texture,
};
use crate::log_ui::{MultilineText, multiline_label};
use crate::map_ui::terrain_name;
use crate::render_context::RenderContext;
use crate::resource_ui::{new_resource_map, resource_name};
use crate::tooltip::show_tooltip_for_circle;
use crate::unit_ui;
use itertools::Itertools;
use macroquad::math::vec2;
use macroquad::prelude::*;
use server::action::Action;
use server::combat_stats::CombatStats;
use server::consts::ARMY_MOVEMENT_REQUIRED_ADVANCE;
use server::content::persistent_events::PersistentEventType;
use server::game::{Game, GameState};
use server::map::block_for_position;
use server::movement::{CurrentMove, MovementAction};
use server::player::Player;
use server::playing_actions::PlayingAction;
use server::position::Position;
use server::resource::ResourceType;
use server::status_phase::get_status_phase;
use server::victory_points::victory_points_parts;

pub(crate) fn player_select(rc: &RenderContext) -> RenderResult {
    let game = rc.game;
    let players = game.human_players_sorted(game.starting_player_index);

    let size = 50.;
    let mut y = (players.len() as f32 * -size) / 2.;

    for player_index in players {
        let pl = game.player(player_index);
        let shown = rc.shown_player.index == pl.index;
        let screen = rc.state.screen_size;
        let pos = vec2(screen.x, screen.y / 2.0) + vec2(-size, y);

        let color = rc.player_color(pl.index);

        let w = if shown { size + 10. } else { size };
        let x = pos.x - w + size;
        let rect = Rect::new(x, pos.y, w, size);
        rc.draw_rectangle(rect, color);
        rc.draw_rectangle_lines(rect, 2.0, BLACK);
        let text = format!("{}", pl.victory_points(game));

        rc.draw_text(&text, pos.x + 10., pos.y + 27.);

        if game.active_player() == pl.index && rc.stage.is_main() {
            draw_texture_ex(
                &rc.assets().active_player,
                x - 20.,
                pos.y + 13.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20., 20.)),
                    ..Default::default()
                },
            );
        }

        let tooltip = if rc.state.control_player.is_some_and(|p| p == pl.index) {
            format!("{pl} (You)")
        } else {
            pl.get_name()
        };
        if button_pressed(rect, rc, &MultilineText::of(rc, &tooltip), 50.) && !shown {
            return StateUpdate::of(StateUpdate::SetShownPlayer(pl.index));
        }

        y += size;
    }

    NO_UPDATE
}

pub(crate) fn top_icon_with_label(
    rc: &RenderContext,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) {
    let dimensions = rc.state.measure_text(label);
    let x = rc.state.screen_size.x / 2.0 + p.x + ((ICON_SIZE - dimensions.width) / 2.0);
    let y = p.y + ICON_SIZE + 30.;
    rc.draw_text(label, x, y);
    top_center_texture(rc, texture, p, tooltip);
}

pub(crate) fn bottom_icon_with_label(
    rc: &RenderContext,
    label: &str,
    texture: &Texture2D,
    p: Vec2,
    tooltip: &str,
) {
    let state = rc.state;
    let dimensions = state.measure_text(label);
    let x = (ICON_SIZE - dimensions.width) / 2.0;
    rc.draw_circle(
        p + bottom_center_anchor(rc) + vec2(15., 20.),
        25.,
        UI_BACKGROUND,
    );

    rc.draw_text(
        label,
        rc.state.screen_size.x / 2.0 + p.x + x,
        rc.state.screen_size.y + p.y + 33.,
    );
    bottom_center_texture(&rc.no_icon_background(), texture, p, tooltip);
}

pub(crate) fn show_top_center(rc: &RenderContext) {
    let player = rc.shown_player;

    rc.draw_rectangle(top_center_rect(rc), UI_BACKGROUND);

    let pos = icon_pos(3, 0);
    top_icon_with_label(
        rc,
        &format!("{}", &player.victory_points(rc.game)),
        &rc.assets().victory_points,
        pos,
        "",
    );

    let mut tooltip = MultilineText::default();
    for (name, points) in victory_points_parts(player, rc.game) {
        tooltip.add(rc, &format!("{name}: {points}"));
    }
    show_tooltip_for_circle(
        rc,
        &tooltip,
        pos + top_center_anchor(rc) + vec2(15., 15.),
        25.,
    );

    top_icon_with_label(
        rc,
        &format!("{}", &player.incident_tokens),
        &rc.assets().event_counter,
        icon_pos(4, 0),
        "Event tokens left",
    );

    let amount = new_resource_map(&player.resources);
    let limit = new_resource_map(&player.resource_limit);
    for (i, r) in ResourceType::all().iter().rev().enumerate() {
        let a = amount[r];
        let l = limit[r];
        let t = if l > 0 {
            format!("{a}/{l}")
        } else {
            format!("{a}")
        };
        top_icon_with_label(
            rc,
            &t,
            &rc.assets().resources[r],
            icon_pos(2 - i as i8, 0),
            resource_name(*r),
        );
    }
}

fn top_center_rect(rc: &RenderContext) -> Rect {
    Rect::new(rc.state.screen_size.x / 2. - 180., 5., 390., 60.)
}

pub(crate) fn show_top_left(rc: &RenderContext, painter: &mut ColumnLabelPainter) {
    let game = rc.game;

    match &game.state {
        GameState::Finished => painter.label("Finished"),
        _ => painter.label(&format!("Age {}", game.age)),
    }
    if let Some(s) = get_status_phase(game) {
        painter.label(&format!("Status Phase: {s}"));
    } else {
        painter.label(&format!("Round {}", game.round));
    }

    let player = rc.shown_player;

    painter.label(&player.get_name());

    painter.label(&format!("Civ {}", player.civilization.name));

    painter.label(&format!(
        "Leader {}",
        if let Some(l) = &player.active_leader() {
            l.name(game)
        } else {
            "-".to_string()
        }
    ));

    if game.current_player_index == player.index {
        if get_status_phase(game).is_none() && game.state != GameState::Finished {
            painter.label(&format!("{} actions left", game.actions_left));
        }
        if let GameState::Movement(moves) = &game.state {
            let movement_actions_left = moves.movement_actions_left;
            painter.label(&format!("Move units: {movement_actions_left} moves left"));
            match moves.current_move {
                CurrentMove::Fleet { .. } => painter.label(
                    "May continue to move the fleet in the same sea without using movement actions",
                ),
                CurrentMove::Embark { .. } => {
                    painter.label("May continue to embark units without using movement actions");
                }
                CurrentMove::None => {}
            }
        }
    }

    if let Some(c) = get_combat(game) {
        if c.attacker.player == player.index {
            painter.label(&format!("Attack - combat round {}", c.round));
        } else if c.defender.player == player.index {
            painter.label(&format!("Defend - combat round {}", c.round));
        }
    }

    if rc.shown_player_is_active() || rc.state.active_dialog.show_for_other_player() {
        for m in rc.state.active_dialog.help_message(rc) {
            painter.label(&m);
        }
    }

    if rc.shown_player_is_active()
        && let Some(u) = &rc.state.pending_update
    {
        for m in &u.info {
            painter.label(m);
        }
    }

    if let Some(position) = rc.state.focused_tile {
        show_focused_tile(painter, game, position);
    }

    if rc.state.show_permanent_effects {
        show_permanent_effects(painter, game, player);
    }
}

pub(crate) struct ColumnLabelPainter<'a> {
    rc: &'a RenderContext<'a>,
    start: Vec2,
    position: Vec2,
    pub background_mode: bool,
    max_column_width: f32,
    used_column_width: f32,
}

impl<'a> ColumnLabelPainter<'a> {
    pub fn new(rc: &'a RenderContext, background_mode: bool) -> Self {
        let p = vec2(10., 10.);
        let mut painter = Self {
            rc,
            position: p,
            start: p,
            background_mode,
            max_column_width: rc.state.screen_size.x / 4.,
            used_column_width: 0.,
        };
        painter.new_column();
        painter
    }

    pub fn label(&mut self, text: &str) {
        let rc = self.rc;
        multiline_label(rc.state, text, self.max_column_width, |label| {
            self.used_column_width = self
                .used_column_width
                .max(rc.state.measure_text(label).width);
            if !self.background_mode {
                rc.draw_text(label, self.position.x, self.position.y + 15.);
            }
            self.position.y += 25.;
        });

        if self.position.y > rc.state.screen_size.y - 150. {
            self.draw_rect();
            self.start = vec2(self.position.x + self.used_column_width + 10., 10.);
            self.new_column();
        }
    }

    fn new_column(&mut self) {
        let center_rect = top_center_rect(self.rc);
        if self.start.x + self.max_column_width > center_rect.x {
            self.start.y = center_rect.y + center_rect.h + 10.;
        }
        self.position = self.start;
    }

    pub fn draw_rect(&mut self) {
        if self.background_mode {
            self.rc.draw_rectangle(
                Rect::new(
                    self.start.x,
                    self.start.y,
                    self.used_column_width,
                    self.position.y - self.start.y,
                ),
                UI_BACKGROUND,
            );
        }
    }
}

fn show_focused_tile(painter: &mut ColumnLabelPainter, game: &Game, position: Position) {
    painter.label(&format!(
        "{}/{}/{}",
        position,
        block_for_position(game, position).0,
        game.map
            .get(position)
            .map_or("outside the map", terrain_name),
    ));

    if let Some(c) = game.try_get_any_city(position) {
        for l in city_labels(game, c) {
            painter.label(&l);
        }
    }

    let units = unit_ui::units_on_tile(game, position).collect_vec();
    if !units.is_empty() {
        painter.label(&format!("Controlled by: {}", game.player_name(units[0].0)));
    }

    for (p, unit) in units {
        let army_move = game.player(p).has_advance(ARMY_MOVEMENT_REQUIRED_ADVANCE);
        painter.label(&unit_ui::unit_label(&unit, army_move, game));
    }
}

fn show_permanent_effects(painter: &mut ColumnLabelPainter, game: &Game, player: &Player) {
    let s = &player.secrets;
    if !s.is_empty() {
        painter.label("Secrets:");
        for e in s {
            painter.label(e);
        }
    }
    painter.label("Permanent effects:");
    for e in &game.permanent_effects {
        for m in e.description(painter.rc.game) {
            painter.label(&m);
        }
    }
}

pub(crate) fn get_combat(game: &Game) -> Option<&CombatStats> {
    game.events.last().and_then(|e| match &e.event_type {
        PersistentEventType::CombatStart(c) => Some(&c.stats),
        PersistentEventType::CombatRoundStart(s) => Some(&s.combat.stats),
        PersistentEventType::CombatRoundEnd(e) => Some(&e.combat.stats),
        PersistentEventType::CombatEnd(s) => Some(s),
        _ => None,
    })
}

pub(crate) fn show_global_controls(rc: &RenderContext, features: &Features) -> RenderResult {
    let assets = rc.assets();
    let can_control = rc.can_control_shown_player();
    if can_control {
        let game = rc.game;
        if let Some(tooltip) = can_end_move(game)
            && bottom_right_texture(rc, &assets.end_turn, icon_pos(-4, -1), tooltip)
        {
            return end_move(game);
        }
        if game.can_redo() && bottom_right_texture(rc, &assets.redo, icon_pos(-5, -1), "Redo") {
            return StateUpdate::execute(Action::Redo);
        }
        if game.can_undo() && bottom_right_texture(rc, &assets.undo, icon_pos(-6, -1), "Undo") {
            return StateUpdate::execute(Action::Undo);
        }

        if can_control {
            action_buttons(rc)?;
        }
    }

    if features.import_export {
        if bottom_right_texture(rc, &assets.export, icon_pos(-1, -3), "Export") {
            return StateUpdate::of(StateUpdate::Export);
        }
        if bottom_right_texture(rc, &assets.import, icon_pos(-2, -3), "Import") {
            return StateUpdate::of(StateUpdate::Import);
        }
    }

    if features.ai {
        let tooltip = if rc.state.ai_autoplay {
            "Pause AI autoplay"
        } else {
            "Start AI autoplay"
        };
        let assets = rc.assets();
        let texture = if rc.state.ai_autoplay {
            &assets.pause
        } else {
            &assets.play
        };
        if bottom_right_texture(rc, texture, icon_pos(-3, -3), tooltip) {
            return StateUpdate::of(StateUpdate::ToggleAiPlay);
        }
    }

    NO_UPDATE
}

fn can_end_move(game: &Game) -> Option<&str> {
    if !game.events.is_empty() {
        return None;
    }
    match game.state {
        GameState::Movement(_) => Some("End movement"),
        GameState::Playing => Some("End turn"),
        GameState::Finished | GameState::ChooseCivilization => None,
    }
}

fn end_move(game: &Game) -> RenderResult {
    if let GameState::Movement(m) = &game.state {
        let movement_actions_left = m.movement_actions_left;
        return StateUpdate::execute_with_warning(
            Action::Movement(MovementAction::Stop),
            if movement_actions_left > 0 {
                vec![format!("{movement_actions_left} movement actions left")]
            } else {
                vec![]
            },
        );
    }

    let left = game.actions_left;
    StateUpdate::execute_with_warning(
        Action::Playing(PlayingAction::EndTurn),
        if left > 0 {
            vec![format!("{left} actions left")]
        } else {
            vec![]
        },
    )
}

pub(crate) fn choose_player_dialog(
    rc: &RenderContext,
    choices: &[usize],
    execute: impl Fn(usize) -> Action,
) -> RenderResult {
    let player = rc.shown_player.index;
    if rc.can_control_active_player() && choices.contains(&player) {
        bottom_centered_text(rc, &format!("Select {}", rc.shown_player.get_name()));
        if ok_button(rc, OkTooltip::Valid("Select".to_string())) {
            return StateUpdate::execute(execute(player));
        }
    }
    NO_UPDATE
}
