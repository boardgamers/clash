use crate::action_buttons::{base_or_custom_action, custom_action_buttons};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::collect_ui::CollectResources;
use crate::construct_ui::{ConstructionPayment, ConstructionProject};
use crate::custom_phase_ui::{SelectedStructureInfo, SelectedStructureStatus};
use crate::happiness_ui::{add_increase_happiness, open_increase_happiness_dialog};
use crate::hex_ui;
use crate::layout_ui::{draw_scaled_icon, is_in_circle};
use crate::map_ui::{move_units_buttons, show_map_action_buttons};
use crate::recruit_unit_ui::RecruitAmount;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use macroquad::math::f32;
use macroquad::prelude::*;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::collect::{available_collect_actions_for_city, possible_resource_collections};
use server::construct::available_buildings;
use server::content::persistent_events::Structure;
use server::game::Game;
use server::playing_actions::PlayingActionType;
use server::resource::ResourceType;
use server::unit::{UnitType, Units};
use std::ops::Add;

pub type IconAction<'a> = (&'a Texture2D, String, Box<dyn Fn() -> StateUpdate + 'a>);

pub type IconActionVec<'a> = Vec<IconAction<'a>>;

pub fn show_city_menu<'a>(rc: &'a RenderContext, city: &'a City) -> StateUpdate {
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
        .filter(|a| increase_happiness_cost(p, city, 1, a).is_some())
        .collect_vec();

    if actions.is_empty() {
        return None;
    }

    Some((
        &rc.assets().resources[&ResourceType::MoodTokens],
        "Increase happiness".to_string(),
        Box::new(move || {
            open_increase_happiness_dialog(rc, actions.clone(), |mut happiness| {
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
    available_buildings(rc.game, rc.shown_player.index, city.position)
        .into_iter()
        .map(|(b, cost_info, pos)| {
            let name = b.name();
            let tooltip = format!(
                "Built {}{} for {}{}",
                name,
                pos.map_or(String::new(), |p| format!(" at {p}")),
                cost_info.cost,
                if cost_info.activate_city {
                    ""
                } else {
                    " (without city activation)"
                }
            );
            let a: IconAction<'a> = (
                &rc.assets().buildings[&b],
                tooltip,
                Box::new(move || {
                    StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(
                        ConstructionPayment::new(
                            rc,
                            city,
                            name,
                            ConstructionProject::Building(b, pos),
                            &cost_info,
                        ),
                    ))
                }),
            );
            a
        })
        .collect()
}

fn recruit_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    if !city.can_activate() || !rc.can_play_action(&PlayingActionType::Recruit) {
        return None;
    }
    Some((
        rc.assets().unit(UnitType::Infantry, rc.shown_player),
        "Recruit Units".to_string(),
        Box::new(|| {
            RecruitAmount::new_selection(
                rc.game,
                city.player_index,
                city.position,
                Units::empty(),
                None,
                &[],
            )
        }),
    ))
}

fn collect_resources_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    let actions = available_collect_actions_for_city(rc.game, rc.shown_player.index, city.position);
    if actions.is_empty() {
        return None;
    }

    Some((
        &rc.assets().resources[&ResourceType::Food],
        "Collect Resources".to_string(),
        Box::new(move || {
            base_or_custom_action(rc, actions.clone(), "Collect resources", |custom| {
                let i = possible_resource_collections(
                    rc.game,
                    city.position,
                    city.player_index,
                    &Vec::new(),
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

pub fn city_labels(game: &Game, city: &City) -> Vec<String> {
    [
        vec![format!(
            "City: {}, {}, {} {}",
            game.player_name(city.player_index),
            city.size(),
            match city.mood_state {
                MoodState::Happy => "Happy",
                MoodState::Neutral => "Neutral",
                MoodState::Angry => "Angry",
            },
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
                        format!("{} (owned by {})", b.name(), game.player_name(*o))
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
) -> Option<StateUpdate> {
    let ActiveDialog::StructuresRequest(d, r) = &rc.state.active_dialog else {
        panic!("Expected StructuresRequest");
    };
    let t = if r.selected.contains(info) {
        HighlightType::Primary
    } else {
        info.highlight_type()
    };
    draw_circle_lines(center.x, center.y, size, 3., t.color());

    if let Some(tooltip) = &info.tooltip {
        show_tooltip_for_circle(rc, tooltip, center, size);
    }

    if info.status != SelectedStructureStatus::Invalid
        && is_mouse_button_pressed(MouseButton::Left)
        && is_in_circle(rc.mouse_pos(), center, size)
    {
        Some(StateUpdate::OpenDialog(ActiveDialog::StructuresRequest(
            d.clone(),
            r.clone().toggle(info.clone()),
        )))
    } else {
        None
    }
}

pub fn draw_city(rc: &RenderContext, city: &City) -> Option<StateUpdate> {
    let c = hex_ui::center(city.position);
    let owner = city.player_index;

    let highlighted = match rc.state.active_dialog {
        ActiveDialog::StructuresRequest(_, ref s) => &s.request.choices,
        _ => &Vec::new(),
    };

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, rc.player_color(owner));

    if let Some(h) = highlighted
        .iter()
        .find(|s| s.position == city.position && matches!(s.structure, Structure::CityCenter))
    {
        if let Some(u) = draw_selected_state(rc, c, 15., h) {
            return Some(u);
        }
    } else {
        draw_mood_state(rc, city, c);
    }

    let i = match draw_wonders(rc, city, c, owner, highlighted) {
        Ok(value) => value,
        Err(value) => return Some(value),
    };

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
            &format!("Happiness: {:?}", city.mood_state),
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
) -> Option<StateUpdate> {
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let p = building_position(city, center, i, *b);
            draw_circle(p.x, p.y, BUILDING_SIZE, rc.player_color(player_index));

            draw_scaled_icon(
                rc,
                &rc.assets().buildings[b],
                b.name(),
                p + vec2(-8., -8.),
                16.,
            );

            if let Some(h) = highlighted.iter().find(|s| {
                s.position == city.position
                    && matches!(s.structure, Structure::Building(bb) if bb == *b)
            }) {
                if let Some(u) = draw_selected_state(rc, p, BUILDING_SIZE, h) {
                    return Some(u);
                }
            }
            i += 1;
        }
    }
    None
}

#[allow(clippy::result_large_err)]
fn draw_wonders(
    rc: &RenderContext,
    city: &City,
    c: Vec2,
    owner: usize,
    highlighted: &[SelectedStructureInfo],
) -> Result<usize, StateUpdate> {
    let mut i = 0;
    for w in &city.pieces.wonders {
        let p = hex_ui::rotate_around(c, 20.0, 90 * i);
        draw_circle(p.x, p.y, 18.0, rc.player_color(owner));
        let size = 20.;
        if let Some(h) = highlighted.iter().find(|s| {
            s.position == city.position
                && matches!(&s.structure, Structure::Wonder(n) if n == &w.name)
        }) {
            if let Some(u) = draw_selected_state(rc, p, 18., h) {
                return Err(u);
            }
        } else {
            draw_scaled_icon(
                rc,
                &rc.assets().wonders[&w.name],
                &w.name,
                p + vec2(-size / 2., -size / 2.),
                size,
            );
        }
        i += 1;
    }
    Ok(i)
}

pub fn building_position(city: &City, center: Vec2, i: usize, building: Building) -> Vec2 {
    if matches!(building, Building::Port) {
        let r: f32 = city
            .position
            .coordinate()
            .directions_to(city.port_position.unwrap().coordinate())[0]
            .to_radians_pointy();
        hex_ui::rotate_around_rad(center, 60.0, r * -1.0 + std::f32::consts::PI / 3.0)
    } else {
        hex_ui::rotate_around(center, 25.0, 90 * i)
    }
}
