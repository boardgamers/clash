use crate::client_state::{ActiveDialog, StateUpdate};
use crate::layout_ui::button_pressed;
use crate::render_context::RenderContext;
use crate::tooltip::add_tooltip_description;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{YELLOW, draw_rectangle};
use server::civilization::Civilization;
use server::content::civilizations;
use std::ops::Mul;

#[derive(Clone)]
pub struct InfoDialog {
    pub select: InfoSelect,
}

impl Default for InfoDialog {
    fn default() -> Self {
        Self::new(InfoSelect::Civilization("Rome".to_string()))
    }
}

impl InfoDialog {
    pub fn new(select: InfoSelect) -> Self {
        InfoDialog { select }
    }
}

#[derive(Clone)]
pub enum InfoSelect {
    Civilization(String),
    // Incident(String),
}

pub fn show_info_dialog(rc: &RenderContext, d: &InfoDialog) -> StateUpdate {
    let p = rc.shown_player;

    draw_button(
        rc,
        "Civilizations",
        vec2(0., 0.),
        &["Show civilization info".to_string()],
    );

    match &d.select {
        InfoSelect::Civilization(selected) => {
            for (i, c) in civilizations::get_all_uncached()
                .iter()
                .filter(|c| c.is_human())
                .enumerate()
            {
                if draw_button(rc, &c.name, vec2(i as f32, 1.), &[]) {
                    return StateUpdate::OpenDialog(ActiveDialog::Info(InfoDialog::new(
                        InfoSelect::Civilization(c.name.clone()),
                    )));
                }
                if c.name == *selected {
                    show_civilization(rc, c);
                }
            }
        }
    }

    StateUpdate::None
}

fn show_civilization(rc: &RenderContext, c: &Civilization) {
    for (i, a) in c.special_advances.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        add_tooltip_description(
            &mut tooltip,
            &format!("Required advance: {}", a.requirement.name(rc.game)),
        );
        add_tooltip_description(&mut tooltip, &a.description);

        draw_button(rc, &a.name, vec2(i as f32, 2.), &tooltip);
    }
}

fn draw_button(rc: &RenderContext, text: &str, pos: Vec2, tooltip: &[String]) -> bool {
    let button_size = vec2(140., 40.);
    let rect_pos = pos.mul(button_size) + vec2(20., 40.);
    let rect = Rect::new(rect_pos.x, rect_pos.y, 135., 30.);

    draw_rectangle(rect.x, rect.y, rect.w, rect.h, YELLOW);

    rc.state.draw_text(text, rect.x + 10., rect.y + 25.);

    button_pressed(rect, rc, tooltip, 50.)
}
