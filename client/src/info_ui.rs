use crate::cards_ui::{action_card_object, objective_card_object, wonder_description};
use crate::city_ui::add_building_description;
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::dialog_ui::{OkTooltip, ok_button};
use crate::event_ui::event_help_tooltip;
use crate::layout_ui::button_pressed;
use crate::log_ui::break_text;
use crate::render_context::RenderContext;
use itertools::Itertools;
use macroquad::color::Color;
use macroquad::math::{Rect, Vec2, vec2};
use macroquad::prelude::{
    BLACK, BLUE, DrawTextureParams, GREEN, MAGENTA, Texture2D, WHITE, YELLOW, draw_texture_ex,
};
use server::action::Action;
use server::action_card::ActionCard;
use server::barbarians::get_barbarians_player;
use server::city_pieces::{BUILDINGS, Building};
use server::civilization::Civilization;
use server::events::EventOrigin;
use server::game::{Game, GameState};
use server::incident::{Incident, IncidentBaseEffect};
use server::leader::Leader;
use server::objective_card::ObjectiveCard;
use server::pirates::get_pirates_player;
use server::resource::ResourceType;
use server::unit::UnitType;
use server::wonder::{Wonder, WonderInfo};
use std::fmt::Display;
use std::ops::Mul;

enum VisibleCardLocation {
    DiscardPile,
    Public,
    Unknown,
}

impl VisibleCardLocation {
    pub(crate) fn from_piles<T: PartialEq>(card: &T, discarded: &[T], public: &[T]) -> Self {
        if discarded.contains(card) {
            VisibleCardLocation::DiscardPile
        } else if public.contains(card) {
            VisibleCardLocation::Public
        } else {
            VisibleCardLocation::Unknown
        }
    }
}

impl Display for VisibleCardLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisibleCardLocation::DiscardPile => write!(f, "Discard Pile"),
            VisibleCardLocation::Public => write!(f, "Public"),
            VisibleCardLocation::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InfoDialog {
    pub select: InfoCategory,
    pub civilization: String,
    pub wonder: Wonder,
    pub incident: u8,
    pub action_card: u8,
    pub objective_card: u8,
    pub unit: UnitType,
    pub building: Building,
}

impl InfoDialog {
    pub(crate) fn choose_civilization(game: &Game) -> Self {
        Self::show_civilization(
            game.cache
                .get_civilizations()
                .iter()
                .find(|c| c.is_used(game).is_none())
                .expect("No unused civilization found")
                .name
                .clone(),
        )
    }

    pub(crate) fn show_civilization(civilization: String) -> Self {
        InfoDialog {
            select: InfoCategory::Civilization,
            civilization,
            wonder: Wonder::Colosseum,
            incident: 1,
            action_card: 1,
            objective_card: 1,
            unit: UnitType::Settler,
            building: Building::Academy,
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
    Buildings,
    Unit,
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
    show_buildings(rc, d)?;
    show_units(rc, d)?;

    NO_UPDATE
}

fn show_wonders(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 1, InfoCategory::Wonder, "Wonders", |rc, d| {
        show_category_items::<WonderInfo, Wonder>(
            rc,
            d,
            |g| g.cache.get_wonders(),
            |w| &w.wonder,
            WonderInfo::name,
            |_, _| None,
            |w| wonder_description(rc, w),
            |d| &d.wonder,
            |d, w| d.wonder = w,
            |w| {
                Some(VisibleCardLocation::from_piles(
                    &w.wonder,
                    &[],
                    &rc.game
                        .players
                        .iter()
                        .flat_map(
                            // todo destroyed wonders
                            |p| p.wonders_built.clone(),
                        )
                        .collect_vec(),
                ))
            },
        )
    })
}

fn show_incidents(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 2, InfoCategory::Incident, "Events", |rc, d| {
        show_category_items::<Incident, u8>(
            rc,
            d,
            |g| g.cache.get_incidents(),
            |i| &i.id,
            |i| i.name.clone(),
            incident_icon,
            |i| event_help_tooltip(rc, &EventOrigin::Incident(i.id)),
            |d| &d.incident,
            |d, i| d.incident = i,
            // todo public from permanent effects
            |i| {
                Some(VisibleCardLocation::from_piles(
                    &i.id,
                    &rc.game.incidents_discarded,
                    &[],
                ))
            },
        )
    })
}

fn incident_icon<'a>(rc: &'a RenderContext, i: &Incident) -> Option<&'a Texture2D> {
    let a = rc.assets();
    match &i.base_effect {
        IncidentBaseEffect::None => None,
        IncidentBaseEffect::BarbariansSpawn => {
            Some(a.unit(UnitType::Infantry, get_barbarians_player(rc.game)))
        }
        IncidentBaseEffect::BarbariansMove => Some(&a.move_units),
        IncidentBaseEffect::PiratesSpawnAndRaid => {
            Some(a.unit(UnitType::Ship, get_pirates_player(rc.game)))
        }
        IncidentBaseEffect::ExhaustedLand => Some(&a.exhausted),
        IncidentBaseEffect::GoldDeposits => Some(&a.resources[&ResourceType::Gold]),
    }
}

fn show_action_cards(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 3, InfoCategory::ActionCard, "Actions", |rc, d| {
        show_category_items::<ActionCard, u8>(
            rc,
            d,
            |g| g.cache.get_action_cards(),
            |i| &i.id,
            ActionCard::name,
            |_, _| None,
            |i| action_card_object(rc, i.id).description,
            |d| &d.action_card,
            |d, i| d.action_card = i,
            |i| {
                Some(VisibleCardLocation::from_piles(
                    &i.id,
                    &rc.game.action_cards_discarded,
                    &[],
                ))
            },
        )
    })
}

fn show_objective_cards(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(
        rc,
        d,
        4,
        InfoCategory::ObjectiveCard,
        "Objectives",
        |rc, d| {
            show_category_items::<ObjectiveCard, u8>(
                rc,
                d,
                |g| g.cache.get_objective_cards(),
                |i| &i.id,
                ObjectiveCard::name,
                |_, _| None,
                |i| objective_card_object(rc, i.id, None).description,
                |d| &d.objective_card,
                |d, i| d.objective_card = i,
                |i| {
                    Some(VisibleCardLocation::from_piles(
                        &i.id,
                        &[],
                        &rc.game
                            .players
                            .iter()
                            .flat_map(|p| p.completed_objectives.iter().map(|o| o.card))
                            .collect_vec(),
                    ))
                },
            )
        },
    )
}

fn show_buildings(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 5, InfoCategory::Buildings, "Buildings", |rc, d| {
        show_category_items::<Building, Building>(
            rc,
            d,
            |_| &BUILDINGS,
            |b| b,
            |b| b.name().to_string(),
            |_, _| None,
            |b| {
                let mut desc = vec![b.name().to_string()];
                let advance = rc.game.cache.get_building_advance(*b).name(rc.game);
                desc.push(format!("Required advance: {advance}"));
                add_building_description(rc, &mut desc, *b);
                desc
            },
            |d| &d.building,
            |d, b| d.building = b,
            |_| None,
        )
    })
}

const UNIT_TYPES: [UnitType; 6] = [
    UnitType::Settler,
    UnitType::Infantry,
    UnitType::Ship,
    UnitType::Cavalry,
    UnitType::Elephant,
    UnitType::Leader(Leader::Alexander), // Placeholder for leaders
];

fn show_units(rc: &RenderContext, d: &InfoDialog) -> RenderResult {
    show_category(rc, d, 6, InfoCategory::Unit, "Units", |rc, d| {
        show_category_items::<UnitType, UnitType>(
            rc,
            d,
            |_| &UNIT_TYPES,
            |u| u,
            |u| {
                if u.is_leader() {
                    "leader"
                } else {
                    u.non_leader_name()
                }
                .to_string()
            },
            |_, _| None,
            |u| {
                let mut parts: Vec<String> = vec![];
                parts.push(u.generic_name().to_string());
                parts.push(format!("Cost: {}", u.cost()));
                if let Some(r) = u.required_building() {
                    parts.push(format!("Required building: {}", r.name()));
                }
                break_text(rc, &mut parts, &u.description());
                parts
            },
            |d| &d.unit,
            |d, u| d.unit = u,
            |_| None,
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
    for (i, c) in rc
        .game
        .cache
        .get_civilizations()
        .iter()
        .filter(|c| c.is_human())
        .sorted_by_key(|c| c.name.clone())
        .enumerate()
    {
        let selected = c.name == d.civilization;
        let color = if let Some(p) = c.is_used(rc.game) {
            rc.player_color(p)
        } else {
            WHITE
        };
        if draw_button_with_color(rc, &c.name, None, vec2(i as f32, 1.), &[], selected, color) {
            let mut new = d.clone();
            new.civilization.clone_from(&c.name);
            return StateUpdate::open_dialog(ActiveDialog::Info(new));
        }
        if selected {
            show_civilization(rc, c)?;
        }
    }
    NO_UPDATE
}

fn show_civilization(rc: &RenderContext, c: &Civilization) -> RenderResult {
    if rc.game.state == GameState::ChooseCivilization
        && ok_button(
            rc,
            if c.is_used(rc.game).is_some() {
                OkTooltip::Invalid(format!("{} is already used by another player", c.name))
            } else {
                OkTooltip::Valid(format!("Choose {}", c.name))
            },
        )
    {
        return StateUpdate::execute_with_confirm(
            vec![format!("Play as {}?", c.name)],
            Action::ChooseCivilization(c.name.clone()),
        );
    }

    for (i, a) in c.special_advances.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        let label = &format!("Name: {}", a.name);
        break_text(rc, &mut tooltip, label);
        let label = &format!("Required advance: {}", a.requirement.name(rc.game));
        break_text(rc, &mut tooltip, label);
        let label = &a.description;
        break_text(rc, &mut tooltip, label);

        draw_button(rc, &a.name, vec2(i as f32, 2.), &tooltip, false);
    }
    for (i, l) in c.leaders.iter().enumerate() {
        let mut tooltip: Vec<String> = vec![];
        let label = &format!("Name: {}", l.name);
        break_text(rc, &mut tooltip, label);
        for a in &l.abilities {
            tooltip.push(format!("Leader ability: {}", a.name));
            let label = &a.description;
            break_text(rc, &mut tooltip, label);
        }

        draw_button(rc, &l.name, vec2(i as f32, 3.), &tooltip, false);
    }
    NO_UPDATE
}

fn show_category_items<T: Clone, K: PartialEq + Ord + Clone>(
    rc: &RenderContext,
    dialog: &InfoDialog,
    get_all: impl Fn(&Game) -> &[T],
    get_key: impl Fn(&T) -> &K,
    name: impl Fn(&T) -> String,
    icon: impl for<'a> Fn(&'a RenderContext<'a>, &T) -> Option<&'a Texture2D>,
    description: impl Fn(&T) -> Vec<String>,
    get_selected: impl Fn(&InfoDialog) -> &K,
    set_selected: impl Fn(&mut InfoDialog, K),
    get_location: impl Fn(&T) -> Option<VisibleCardLocation>,
) -> RenderResult {
    for (i, info) in get_all(rc.game)
        .iter()
        .sorted_by_key(|info| get_key(info))
        .enumerate()
    {
        let selected = get_key(info) == get_selected(dialog);
        let name = name(info);
        let mut desc = description(info);
        let location = get_location(info);
        if let Some(l) = &location {
            desc.push(format!("Location: {l}"));
        }
        let columns = 7;
        let rect = vec2(i.rem_euclid(columns) as f32, ((i / columns) + 1) as f32);
        if draw_button_with_color(
            rc,
            &name,
            icon(rc, info),
            rect,
            &desc,
            selected,
            match location {
                Some(VisibleCardLocation::DiscardPile) => BLUE,
                Some(VisibleCardLocation::Public) => YELLOW,
                Some(VisibleCardLocation::Unknown) | None => GREEN,
            },
        ) {
            let mut new = dialog.clone();
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
    draw_button_with_color(rc, text, None, pos, tooltip, selected, color)
}

fn draw_button_with_color(
    rc: &RenderContext,
    text: &str,
    icon: Option<&Texture2D>,
    pos: Vec2,
    tooltip: &[String],
    selected: bool,
    color: Color,
) -> bool {
    let r = button_rect(pos);

    if rc.stage.is_main() {
        rc.draw_rectangle(r, color);
        let mut w = r.w - 30.;

        if let Some(icon) = icon {
            draw_texture_ex(
                icon,
                r.x + 105.,
                r.y + 5.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20., 20.)),
                    ..Default::default()
                },
            );
            w -= 25.;
        }

        rc.draw_limited_text(text, r.x + 10., r.y + 22., w);
    }

    if selected {
        rc.draw_rectangle_lines(r, 4., BLACK);
    }

    button_pressed(r, rc, tooltip, 50.)
}

fn button_rect(pos: Vec2) -> Rect {
    let button_size = vec2(140., 40.);
    let rect_pos = pos.mul(button_size) + vec2(20., 40.);

    Rect::new(rect_pos.x, rect_pos.y, 135., 30.)
}
