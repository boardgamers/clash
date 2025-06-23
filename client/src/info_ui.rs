use crate::cards_ui::{action_card_object, objective_card_object, wonder_description};
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::layout_ui::button_pressed;
use crate::log_ui::{break_each, break_text};
use crate::render_context::RenderContext;
use itertools::Itertools;
use macroquad::color::Color;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{BLACK, BLUE, GREEN, MAGENTA, WHITE, YELLOW};
use server::action_card::ActionCard;
use server::civilization::Civilization;
use server::content::civilizations;
use server::game::Game;
use server::incident::Incident;
use server::objective_card::ObjectiveCard;
use server::wonder::{Wonder, WonderInfo};
use std::ops::Mul;

#[derive(Clone, Debug)]
pub(crate) struct InfoDialog {
    pub select: InfoCategory,
    pub civilization: String,
    pub wonder: Wonder,
    pub incident: u8,
    pub action_card: u8,
    pub objective_card: u8,
}

impl InfoDialog {
    pub(crate) fn new(civilization: String) -> Self {
        InfoDialog {
            select: InfoCategory::Civilization,
            civilization,
            wonder: Wonder::Colosseum,
            incident: 1,
            action_card: 1,
            objective_card: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InfoCategory {
    Civilization,
    Wonder,
    Incident,
    ActionCard,
    ObjectiveCard,
}

pub(crate) fn show_info_dialog(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(
        rc,
        d,
        0,
        InfoCategory::Civilization,
        "Civilizations",
        show_civilizations,
    )?;
    show_wonders(rc, d)?;
    show_incidents(rc, d)?;
    show_action_cards(rc, d)?;
    show_objective_cards(rc, d)?;

    NO_UPDATE
}

fn show_incidents(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 2, InfoCategory::Incident, "Events", |rc, d| {
        show_category_items::<Incident, u8>(
            rc,
            d,
            |g| g.cache.get_incidents(),
            |i| &i.id,
            |i| i.name.clone(),
            |i| {
                let mut d: Vec<String> = vec![];
                break_each(&mut d, &i.description(rc.game));
                d
            },
            |d| &d.incident,
            |d, i| d.incident = i,
        )
    })
}

fn show_action_cards(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(
        rc,
        d,
        3,
        InfoCategory::ActionCard,
        "Action Cards",
        |rc, d| {
            show_category_items::<ActionCard, u8>(
                rc,
                d,
                |g| g.cache.get_action_cards(),
                |i| &i.id,
                ActionCard::name,
                |i| action_card_object(rc, i.id).description,
                |d| &d.action_card,
                |d, i| d.action_card = i,
            )
        },
    )
}

fn show_objective_cards(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(
        rc,
        d,
        4,
        InfoCategory::ObjectiveCard,
        "Objective Cards",
        |rc, d| {
            show_category_items::<ObjectiveCard, u8>(
                rc,
                d,
                |g| g.cache.get_objective_cards(),
                |i| &i.id,
                ObjectiveCard::name,
                |i| objective_card_object(rc, i.id, None).description,
                |d| &d.objective_card,
                |d, i| d.objective_card = i,
            )
        },
    )
}

fn show_wonders(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 1, InfoCategory::Wonder, "Wonders", |rc, d| {
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
    })
}

fn show_category(
    rc: &RenderContext,
    d: &InfoDialog,
    x: usize,
    category: InfoCategory,
    name: &str,
    show: impl Fn(&RenderContext, &InfoDialog) -> RenderResult,
) -> RenderResult {
    let pos = vec2(x as f32, 0.);
    let selected = d.select == category;
    if draw_button(rc, name, pos, &[], selected) {
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
        .sorted_by_key(|c| c.name.clone())
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
        let label = &format!("Name: {}", a.name);
        break_text(&mut tooltip, label);
        let label = &format!("Required advance: {}", a.requirement.name(rc.game));
        break_text(&mut tooltip, label);
        let label = &a.description;
        break_text(&mut tooltip, label);

        draw_button(rc, &a.name, vec2(i as f32, 2.), &tooltip, false);
    }
    for (i, l) in c.leaders.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        let label = &format!("Name: {}", l.name);
        break_text(&mut tooltip, label);
        for a in &l.abilities {
            tooltip.push(format!("Leader ability: {}", a.name));
            let label = &a.description;
            break_text(&mut tooltip, label);
        }

        draw_button(rc, &l.name, vec2(i as f32, 3.), &tooltip, false);
    }
}

fn show_category_items<T: Clone, K: PartialEq + Ord + Clone>(
    rc: &RenderContext,
    d: &InfoDialog,
    get_all: impl Fn(&Game) -> &Vec<T>,
    get_key: impl Fn(&T) -> &K,
    name: impl Fn(&T) -> String,
    description: impl Fn(&T) -> Vec<String>,
    get_selected: impl Fn(&InfoDialog) -> &K,
    set_selected: impl Fn(&mut InfoDialog, K),
) -> RenderResult {
    for (i, info) in get_all(rc.game)
        .iter()
        .sorted_by_key(|info| get_key(info))
        .enumerate()
    {
        let selected = get_key(info) == get_selected(d);
        let name = name(info);
        if draw_button_with_color(
            rc,
            &name,
            vec2(i.rem_euclid(8) as f32, ((i / 8) + 1) as f32),
            &description(info),
            selected,
            GREEN,
        ) {
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
    let color = match pos.y {
        0. => YELLOW,
        1. => GREEN,
        2. => BLUE,
        3. => MAGENTA,
        _ => WHITE,
    };
    draw_button_with_color(rc, text, pos, tooltip, selected, color)
}

fn draw_button_with_color(
    rc: &RenderContext,
    text: &str,
    pos: Vec2,
    tooltip: &[String],
    selected: bool,
    color: Color,
) -> bool {
    let button_size = vec2(140., 40.);
    let rect_pos = pos.mul(button_size) + vec2(20., 40.);
    let rect = Rect::new(rect_pos.x, rect_pos.y, 135., 30.);

    rc.draw_rectangle_with_text(rect, color, text);

    if selected {
        rc.draw_rectangle_lines(rect, 4., BLACK);
    }

    button_pressed(rect, rc, tooltip, 50.)
}
