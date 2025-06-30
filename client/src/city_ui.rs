use crate::action_buttons::{base_or_custom_action, custom_action_buttons};
use crate::client_state::{ActiveDialog, NO_UPDATE, RenderResult, StateUpdate};
use crate::collect_ui::CollectResources;
use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::custom_phase_ui::{SelectedStructureInfo, SelectedStructureStatus};
use crate::happiness_ui::{
    add_increase_happiness, available_happiness_actions_for_city, can_afford_increase_happiness,
    open_increase_happiness_dialog,
};
use crate::hex_ui;
use crate::layout_ui::{
    draw_scaled_icon, draw_scaled_icon_with_tooltip, is_in_circle, is_mouse_pressed,
};
use crate::log_ui::break_text;
use crate::map_ui::{move_units_buttons, show_map_action_buttons};
use crate::recruit_unit_ui::RecruitAmount;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use macroquad::math::f32;
use macroquad::prelude::*;
use server::city::{City, MoodState};
use server::city_pieces::{BUILDINGS, Building};
use server::collect::{available_collect_actions_for_city, possible_resource_collections};
use server::construct::{
    BUILDING_ALREADY_EXISTS, NOT_ENOUGH_RESOURCES, can_construct, new_building_positions,
};
use server::consts::BUILDING_COST;
use server::events::check_event_origin;
use server::game::Game;
use server::player::CostTrigger;
use server::playing_actions::PlayingActionType;
use server::resource::ResourceType;
use server::structure::Structure;
use server::unit::{UnitType, Units};
use std::ops::Add;

pub(crate) struct IconAction<'a> {
    pub texture: &'a Texture2D,
    pub skip_background: bool,
    pub tooltip: Vec<String>,
    pub warning: Option<HighlightType>,
    pub action: Box<dyn Fn() -> RenderResult + 'a>,
}

impl<'a> IconAction<'a> {
    #[must_use]
    pub(crate) fn new(
        texture: &'a Texture2D,
        skip_background: bool,
        tooltip: Vec<String>,
        action: Box<dyn Fn() -> RenderResult + 'a>,
    ) -> IconAction<'a> {
        IconAction {
            texture,
            skip_background,
            tooltip,
            warning: None,
            action,
        }
    }

    #[must_use]
    pub(crate) fn with_warning(self, warning: Option<HighlightType>) -> IconAction<'a> {
        IconAction { warning, ..self }
    }

    #[must_use]
    pub(crate) fn with_rc(
        &self,
        rc: &RenderContext,
        button: impl Fn(&RenderContext) -> bool,
    ) -> bool {
        if self.skip_background {
            button(&rc.no_icon_background())
        } else {
            button(rc)
        }
    }
}

pub(crate) type IconActionVec<'a> = Vec<IconAction<'a>>;

pub(crate) fn show_city_menu<'a>(rc: &'a RenderContext, city: &'a City) -> RenderResult {
    let base_icons: IconActionVec<'a> = vec![
        increase_happiness_button(rc, city),
        collect_resources_button(rc, city),
        recruit_button(rc, city),
    ]
    .into_iter()
    .flatten()
    .collect();

    let icons = vec![
        base_icons,
        move_units_buttons(rc, city.position),
        building_icons(rc, city),
        custom_action_buttons(rc, Some(city)),
    ]
    .into_iter()
    .flatten()
    .collect();
    show_map_action_buttons(rc, &icons)
}

fn increase_happiness_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    let p = rc.shown_player;
    let actions = available_happiness_actions_for_city(rc.game, p.index, city.position)
        .into_iter()
        .filter(|a| can_afford_increase_happiness(rc, city, 1, a))
        .collect_vec();

    if actions.is_empty() {
        return None;
    }

    Some(IconAction::new(
        &rc.assets().resources[&ResourceType::MoodTokens],
        false,
        vec!["Increase happiness".to_string()],
        Box::new(move || {
            open_increase_happiness_dialog(rc, &actions, |mut happiness| {
                let mut target = city.mood_state.clone();
                while target != MoodState::Happy {
                    happiness = add_increase_happiness(rc, city, happiness)
                        .expect("Happiness action failed");
                    target = target.clone().add(1);
                }
                happiness
            })
        }),
    ))
}

fn building_icons<'a>(rc: &'a RenderContext, city: &'a City) -> IconActionVec<'a> {
    if !city.can_activate() || !rc.can_play_action(&PlayingActionType::Construct) {
        return vec![];
    }
    let game = rc.game;
    BUILDINGS
        .into_iter()
        .flat_map(|b| {
            let can = can_construct(
                city,
                b,
                rc.shown_player,
                game,
                CostTrigger::WithModifiers,
                &[],
            );

            new_building_positions(game, b, city)
                .into_iter()
                .map(|pos| (b, can.clone(), pos))
                .collect_vec()
        })
        .map(|(b, can, pos)| {
            let name = b.name();
            let warn = can.as_ref().err().map(|e| {
                if e == NOT_ENOUGH_RESOURCES {
                    HighlightType::NotEnoughResources
                } else if e == BUILDING_ALREADY_EXISTS {
                    HighlightType::AlreadyExists
                } else if e.contains("Missing advance") {
                    HighlightType::MissingAdvance
                } else {
                    HighlightType::Warn
                }
            });

            let suffix = match &can {
                Ok(c) => format!(
                    " for {}{}",
                    c.cost,
                    if c.activate_city {
                        ""
                    } else {
                        " (without city activation)"
                    }
                ),
                Err(e) => format!(" ({e})"),
            };
            let tooltip = vec![
                format!(
                    "{}{}{}",
                    name,
                    pos.map_or(String::new(), |p| format!(" at {p}")),
                    suffix,
                ),
                b.description().to_string(),
            ];
            IconAction::new(
                &rc.assets().buildings[&b],
                true,
                tooltip,
                Box::new(move || {
                    can.clone().map_or(NO_UPDATE, |cost_info| {
                        StateUpdate::open_dialog(ActiveDialog::ConstructionPayment(
                            ConstructionPayment::new(
                                rc,
                                city,
                                name,
                                ConstructionProject::Building(b, pos),
                                &cost_info,
                            ),
                        ))
                    })
                }),
            )
            .with_warning(warn)
        })
        .collect()
}

fn recruit_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    if !city.can_activate() || !rc.can_play_action(&PlayingActionType::Recruit) {
        return None;
    }
    Some(IconAction::new(
        rc.assets().unit(UnitType::Infantry, rc.shown_player),
        false,
        vec!["Recruit Units".to_string()],
        Box::new(|| {
            RecruitAmount::new_selection(rc.game, city.player_index, city.position, Units::empty())
        }),
    ))
}

fn collect_resources_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    let actions = available_collect_actions_for_city(rc.game, rc.shown_player.index, city.position);
    if actions.is_empty() {
        return None;
    }

    Some(IconAction::new(
        &rc.assets().resources[&ResourceType::Food],
        false,
        vec!["Collect Resources".to_string()],
        Box::new(move || {
            base_or_custom_action(rc, &actions, "Collect resources", |custom| {
                let i = possible_resource_collections(
                    rc.game,
                    city.position,
                    city.player_index,
                    &check_event_origin(),
                    CostTrigger::WithModifiers,
                );
                ActiveDialog::CollectResources(CollectResources::new(
                    city.player_index,
                    city.position,
                    custom,
                    i,
                ))
            })
        }),
    ))
}

pub(crate) fn city_labels(game: &Game, city: &City) -> Vec<String> {
    [
        vec![format!(
            "City: {}, {}, {} {}",
            game.player_name(city.player_index),
            city.size(),
            city.mood_state,
            if city.is_activated() {
                " (activated)"
            } else {
                ""
            },
        )],
        city.pieces
            .building_owners()
            .iter()
            .filter_map(|(b, o)| {
                o.as_ref().map(|o| {
                    if city.player_index == *o {
                        b.name().to_string()
                    } else {
                        format!("{b} (owned by {})", game.player_name(*o))
                    }
                })
            })
            .collect(),
    ]
    .concat()
}

pub const BUILDING_SIZE: f32 = 12.0;

fn draw_selected_state(
    rc: &RenderContext,
    center: Vec2,
    size: f32,
    info: &SelectedStructureInfo,
) -> RenderResult {
    let ActiveDialog::StructuresRequest(d, r) = &rc.state.active_dialog else {
        panic!("Expected StructuresRequest");
    };
    let t = if r.selected.contains(info) {
        HighlightType::Primary
    } else {
        info.highlight_type()
    };
    rc.draw_circle_lines(center, size, 3., t.color());

    if let Some(tooltip) = &info.tooltip {
        show_tooltip_for_circle(rc, &[tooltip.clone()], center, size);
    }

    if info.status != SelectedStructureStatus::Invalid
        && is_mouse_pressed(rc)
        && is_in_circle(rc.mouse_pos(), center, size)
    {
        StateUpdate::open_dialog(ActiveDialog::StructuresRequest(
            d.clone(),
            r.clone().toggle(info.clone()),
        ))
    } else {
        NO_UPDATE
    }
}

pub(crate) fn draw_city(rc: &RenderContext, city: &City) -> RenderResult {
    let c = hex_ui::center(city.position);
    let owner = city.player_index;

    let highlighted = match rc.state.active_dialog {
        ActiveDialog::StructuresRequest(_, ref s) => &s.request.choices,
        _ => &Vec::new(),
    };

    if city.is_activated() {
        rc.draw_circle(c, 18.0, WHITE);
    }
    rc.draw_circle(c, 15.0, rc.player_color(owner));

    if let Some(h) = highlighted
        .iter()
        .find(|s| s.position == city.position && matches!(s.structure, Structure::CityCenter))
    {
        draw_selected_state(rc, c, 15., h)?;
    } else {
        draw_mood_state(rc, city, c);
    }

    let i = draw_wonders(rc, city, c, owner, highlighted)?;
    draw_buildings(rc, city, c, highlighted, i)
}

fn draw_mood_state(rc: &RenderContext, city: &City, c: Vec2) {
    let state = &rc.state;
    let mood = match &state.active_dialog {
        ActiveDialog::IncreaseHappiness(increase) => {
            let steps = increase
                .steps
                .iter()
                .find(|(p, _)| p == &city.position)
                .map_or(&0, |(_, s)| s);
            &city.mood_state.clone().add(*steps)
        }
        _ => &city.mood_state,
    };
    let t = match mood {
        MoodState::Happy => Some(&rc.assets().resources[&ResourceType::MoodTokens]),
        MoodState::Neutral => None,
        MoodState::Angry => Some(&rc.assets().angry),
    };
    if let Some(t) = t {
        let size = 15.;
        draw_scaled_icon(
            rc,
            t,
            &format!("Happiness: {}", city.mood_state),
            c + vec2(-size / 2., -size / 2.),
            size,
        );
    }
}

fn draw_buildings(
    rc: &RenderContext,
    city: &City,
    center: Vec2,
    highlighted: &[SelectedStructureInfo],
    mut i: usize,
) -> RenderResult {
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let p = building_position(city, center, i, *b);
            rc.draw_circle(p, BUILDING_SIZE, rc.player_color(player_index));

            let mut tooltip = vec![b.name().to_string()];
            add_building_description(rc, &mut tooltip, *b);

            draw_scaled_icon_with_tooltip(
                rc,
                &rc.assets().buildings[b],
                &tooltip,
                p + vec2(-8., -8.),
                16.,
            );

            if let Some(h) = highlighted.iter().find(|s| {
                s.position == city.position
                    && matches!(s.structure, Structure::Building(bb) if bb == *b)
            }) {
                draw_selected_state(rc, p, BUILDING_SIZE, h)?;
            }
            i += 1;
        }
    }
    NO_UPDATE
}

pub(crate) fn add_building_description(rc: &RenderContext, parts: &mut Vec<String>, b: Building) {
    let pile = rc
        .shown_player
        .building_cost(rc.game, b, CostTrigger::WithModifiers)
        .cost
        .first_valid_payment(&BUILDING_COST)
        .expect("Building cost should be valid");
    if pile == BUILDING_COST {
        parts.push(format!("Cost: {BUILDING_COST}"));
    } else {
        parts.push(format!("Base cost: {BUILDING_COST}"));
        parts.push(format!("Current cost: {pile}"));
    }
    break_text(rc, parts, b.description());
}

#[allow(clippy::result_large_err)]
fn draw_wonders(
    rc: &RenderContext,
    city: &City,
    c: Vec2,
    owner: usize,
    highlighted: &[SelectedStructureInfo],
) -> Result<usize, Box<StateUpdate>> {
    let mut i = 0;
    for w in &city.pieces.wonders {
        let p = hex_ui::rotate_around(c, 20.0, 90 * i);
        rc.draw_circle(p, 18.0, rc.player_color(owner));
        let size = 20.;
        if let Some(h) = highlighted.iter().find(|s| {
            s.position == city.position && matches!(&s.structure, Structure::Wonder(n) if n == w)
        }) {
            draw_selected_state(rc, p, 18., h)?;
        } else {
            draw_scaled_icon(
                rc,
                &rc.assets().wonders[w],
                &w.name(),
                p + vec2(-size / 2., -size / 2.),
                size,
            );
        }
        i += 1;
    }
    Ok(i)
}

pub(crate) fn building_position(city: &City, center: Vec2, i: usize, building: Building) -> Vec2 {
    if matches!(building, Building::Port) {
        let r: f32 = city
            .position
            .coordinate()
            .directions_to(city.port_position.unwrap().coordinate())[0]
            .to_radians_pointy();
        hex_ui::rotate_around_rad(center, 60.0, -r + std::f32::consts::PI / 3.0)
    } else {
        hex_ui::rotate_around(center, 25.0, 90 * i)
    }
}
