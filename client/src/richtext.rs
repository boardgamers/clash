use crate::city_ui::draw_mood_state;
use crate::layout_ui::{FONT_SIZE, draw_scaled_icon};
use crate::render_context::RenderContext;
use macroquad::color::{BLACK, Color};
use macroquad::math::{Vec2, vec2};
use macroquad::prelude::Texture2D;
use server::city::MoodState;
use server::log::ActionLogBalance;
use server::position::Position;
use server::resource_pile::ResourcePile;
use server::structure::Structure;
use server::unit::{UnitType, Units};
use server::wonder::Wonder;

pub struct RichTextDrawer<'a> {
    pub(crate) rc: &'a RenderContext<'a>,
    pub(crate) current_pos: Vec2,
    space: f32,
}

impl RichTextDrawer<'_> {
    pub(crate) fn new<'a>(rc: &'a RenderContext<'_>, start_pos: Vec2) -> RichTextDrawer<'a> {
        RichTextDrawer {
            rc,
            current_pos: start_pos,
            space: rc.state.measure_text(" ").width,
        }
    }

    pub(crate) fn text(&mut self, text: &str) {
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

    pub(crate) fn modifier(&mut self, modifier: &str, balance: Option<&ActionLogBalance>) {
        let verb = match balance {
            None | Some(ActionLogBalance::Gain) => "using",
            Some(ActionLogBalance::Loss) => "to",
            Some(ActionLogBalance::Pay) => "for",
        };
        self.text(&format!("{verb} {modifier}"));
    }

    pub(crate) fn icon(&mut self, texture: &Texture2D) {
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

    pub(crate) fn player(&mut self, player: usize) {
        self.text_ex(
            &self.rc.game.player_name(player),
            self.rc.player_color(player),
            FONT_SIZE,
        );
        self.current_pos.x += self.space;
    }

    pub(crate) fn at_location(&mut self, position: Position) {
        self.text("at");
        self.location(position);
    }

    pub(crate) fn location(&mut self, position: Position) {
        self.icon_with_size(&self.rc.assets().hex, 35.0);
        self.current_pos.x -= 31.0;
        self.text_ex(&format!("{position}"), BLACK, 17);
    }

    pub(crate) fn resources(&mut self, resources: &ResourcePile) {
        for (resource, amount) in resources.clone() {
            if amount > 0
                && let Some(texture) = self.rc.assets().resources.get(&resource)
            {
                self.icon(texture);
                self.text(&amount.to_string());
            }
        }
    }

    pub(crate) fn unit_icon(&mut self, unit_type: UnitType) {
        self.icon(self.rc.assets().unit(unit_type, self.rc.shown_player));
    }

    pub(crate) fn units(&mut self, units: &Units) {
        for (unit, amount) in units.clone() {
            if amount > 0 {
                self.unit_icon(unit);
                let mut u = Units::empty();
                for _ in 0..amount {
                    u += &unit;
                }
                self.text(&u.to_string(Some(self.rc.game)));
            }
        }
    }

    pub(crate) fn action_icon(&mut self) {
        self.icon(&self.rc.assets().end_turn);
    }

    pub(crate) fn structure(
        &mut self,
        structure: &Structure,
        position: Position,
        port_position: Option<Position>,
        player: usize,
    ) {
        // Draw structure icon or circle
        match structure {
            Structure::CityCenter => {
                self.rc.draw_circle(
                    center(self.current_pos),
                    RADIUS,
                    self.rc.player_color(player),
                );
                self.current_pos.x += 35.0;
                self.text("City");
            }
            Structure::Building(b) => {
                draw_scaled_icon(
                    self.rc,
                    &self.rc.assets().buildings[b],
                    "",
                    center(self.current_pos),
                    RADIUS,
                );
                self.current_pos.x += 35.0;
                self.text(b.name());
            }
            Structure::Wonder(w) => {
                self.wonder(*w);
            }
        }
        self.at_location(position);
        if let Some(port_pos) = port_position {
            self.text(" at the water tile");
            self.at_location(port_pos);
        }
    }

    pub(crate) fn wonder(&mut self, w: Wonder) {
        draw_scaled_icon(
            self.rc,
            &self.rc.assets().wonders[&w],
            "",
            center(self.current_pos),
            RADIUS,
        );
        self.current_pos.x += 35.0;
        self.text(&w.name());
    }

    pub(crate) fn mood(&mut self, player: usize, mood: &MoodState) {
        let c = center(self.current_pos);
        self.rc.draw_circle(c, RADIUS, self.rc.player_color(player));
        draw_mood_state(self.rc, c, mood);
        self.current_pos.x += 35.0;
    }
}

const RADIUS: f32 = 15.0;

fn center(message_pos: Vec2) -> Vec2 {
    vec2(message_pos.x + 15.0, message_pos.y - RADIUS / 2.0)
}
