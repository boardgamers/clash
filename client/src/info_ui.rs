use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::layout_ui::button_pressed;
use crate::render_context::RenderContext;
use crate::tooltip::add_tooltip_description;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{BLACK, BLUE, GREEN, MAGENTA, WHITE, YELLOW};
use server::civilization::Civilization;
use server::content::civilizations;
use std::ops::Mul;

#[derive(Clone, Debug)]
pub(crate) struct InfoDialog {
    pub select: InfoSelect,
}

impl InfoDialog {
    pub(crate) fn new(select: InfoSelect) -> Self {
        InfoDialog { select }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InfoSelect {
    Civilization(String),
    // Incident(String),
}

pub(crate) fn show_info_dialog(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    draw_button(
        rc,
        "Civilizations",
        vec2(0., 0.),
        &["Show civilization info".to_string()],
        true,
    );

    match &d.select {
        InfoSelect::Civilization(civ) => {
            for (i, c) in civilizations::get_all_uncached()
                .iter()
                .filter(|c| c.is_human())
                .enumerate()
            {
                let selected = &c.name == civ;
                if draw_button(rc, &c.name, vec2(i as f32, 1.), &[], selected) {
                    return StateUpdate::open_dialog(ActiveDialog::Info(InfoDialog::new(
                        InfoSelect::Civilization(c.name.clone()),
                    )));
                }
                if selected {
                    show_civilization(rc, c);
                }
            }
        }
    }

    NO_UPDATE
}

fn show_civilization(rc: &RenderContext, c: &Civilization) {
    for (i, a) in c.special_advances.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        add_tooltip_description(&mut tooltip, &format!("Name: {}", a.name));
        add_tooltip_description(
            &mut tooltip,
            &format!("Required advance: {}", a.requirement.name(rc.game)),
        );
        add_tooltip_description(&mut tooltip, &a.description);

        draw_button(rc, &a.name, vec2(i as f32, 2.), &tooltip, false);
    }
}

fn draw_button(
    rc: &RenderContext,
    text: &str,
    pos: Vec2,
    tooltip: &[String],
    selected: bool,
) -> bool {
    let button_size = vec2(140., 40.);
    let rect_pos = pos.mul(button_size) + vec2(20., 40.);
    let rect = Rect::new(rect_pos.x, rect_pos.y, 135., 30.);

    let color = match pos.y {
        0. => YELLOW,
        1. => GREEN,
        2. => BLUE,
        3. => MAGENTA,
        _ => WHITE,
    };
    rc.draw_rectangle(rect, color);

    if selected {
        rc.draw_rectangle_lines(rect, 4., BLACK);
    }

    rc.draw_limited_text(text, rect.x + 10., rect.y + 25., 14);

    button_pressed(rect, rc, tooltip, 50.)
}
