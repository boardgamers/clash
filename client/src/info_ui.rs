use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::layout_ui::button_pressed;
use crate::render_context::RenderContext;
use crate::tooltip::add_tooltip_description;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{BLACK, BLUE, GREEN, MAGENTA, WHITE, YELLOW};
use server::civilization::Civilization;
use server::content::civilizations;
use server::game::Game;
use server::wonder::{Wonder, WonderInfo};
use std::ops::Mul;
use crate::cards_ui::wonder_description;

#[derive(Clone, Debug)]
pub(crate) struct InfoDialog {
    pub select: InfoCategory,
    pub civilization: String,
    pub wonder: Wonder,
}

impl InfoDialog {
    pub(crate) fn new(civilization: String) -> Self {
        InfoDialog {
            select: InfoCategory::Civilization,
            civilization,
            wonder: Wonder::GreatWall,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InfoCategory {
    Civilization,
    Wonder,
}

pub(crate) fn show_info_dialog(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(
        rc,
        d,
        0,
        InfoCategory::Civilization,
        "Civilizations",
        "Show civilization info",
        show_civilizations,
    )?;
    show_category(
        rc,
        d,
        1,
        InfoCategory::Wonder,
        "Wonders",
        "Show wonder info",
        |rc, d| {
            show_category_items::<WonderInfo, Wonder>(
                rc,
                d,
                |g| g.cache.get_wonders(),
                |w| &w.wonder,
                WonderInfo::name,
                |w| wonder_description(rc, w),
                |d| &d.wonder,
                |d, w| d.wonder = w,
            )
        },
    )?;

    NO_UPDATE
}

fn show_category(
    rc: &RenderContext,
    d: &InfoDialog,
    x: usize,
    category: InfoCategory,
    name: &str,
    tooltip: &str,
    show: impl Fn(&RenderContext, &InfoDialog) -> RenderResult,
) -> RenderResult {
    let pos = vec2(x as f32, 0.);
    let selected = d.select == category;
    if draw_button(rc, name, pos, &[tooltip.to_string()], selected) {
        let mut new = d.clone();
        new.select = category;
        return StateUpdate::open_dialog(ActiveDialog::Info(new));
    }
    if selected {
        show(rc, d)?;
    }
    NO_UPDATE
}

fn show_civilizations(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    for (i, c) in civilizations::get_all_uncached()
        .iter()
        .filter(|c| c.is_human())
        .enumerate()
    {
        let selected = c.name == d.civilization;
        if draw_button(rc, &c.name, vec2(i as f32, 1.), &[], selected) {
            let mut new = d.clone();
            new.civilization.clone_from(&c.name);
            return StateUpdate::open_dialog(ActiveDialog::Info(new));
        }
        if selected {
            show_civilization(rc, c);
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
    for (i, l) in c.leaders.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        add_tooltip_description(&mut tooltip, &format!("Name: {}", l.name));
        for a in &l.abilities {
            tooltip.push(format!("Leader ability: {}", a.name));
            add_tooltip_description(&mut tooltip, &a.description);
        }

        draw_button(rc, &l.name, vec2(i as f32, 3.), &tooltip, false);
    }
}

fn show_category_items<T: Clone, K: PartialEq + Clone>(
    rc: &RenderContext,
    d: &InfoDialog,
    get_all: impl Fn(&Game) -> &Vec<T>,
    get_key: impl Fn(&T) -> &K,
    name: impl Fn(&T) -> String,
    description: impl Fn(&T) -> Vec<String>,
    get_selected: impl Fn(&InfoDialog) -> &K,
    set_selected: impl Fn(&mut InfoDialog, K),
) -> RenderResult {
    for (i, info) in get_all(rc.game).iter().enumerate() {
        let selected = get_key(info) == get_selected(d);
        let name = name(info);
        if draw_button(rc, &name, vec2(i as f32, 1.), &description(info), selected) {
            let mut new = d.clone();
            set_selected(&mut new, get_key(info).clone());
            return StateUpdate::open_dialog(ActiveDialog::Info(new));
        }
    }
    NO_UPDATE
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
    rc.draw_rectangle_with_text(rect, color, text);

    if selected {
        rc.draw_rectangle_lines(rect, 4., BLACK);
    }

    button_pressed(rect, rc, tooltip, 50.)
}
