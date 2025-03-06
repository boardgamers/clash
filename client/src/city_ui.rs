use crate::action_buttons::{
    base_or_custom_action, base_or_custom_available, custom_action_buttons,
};
use crate::client_state::{ActiveDialog, StateUpdate};
use crate::collect_ui::CollectResources;
use crate::construct_ui::{new_building_positions, ConstructionPayment, ConstructionProject};
use crate::custom_phase_ui::{highlight_structures, StructureHighlight};
use crate::happiness_ui::{
    add_increase_happiness, can_play_increase_happiness, open_increase_happiness_dialog,
};
use crate::hex_ui;
use crate::layout_ui::{draw_scaled_icon, is_in_circle};
use crate::map_ui::{move_units_buttons, show_map_action_buttons};
use crate::recruit_unit_ui::RecruitAmount;
use crate::render_context::RenderContext;
use crate::select_ui::HighlightType;
use itertools::Itertools;
use macroquad::prelude::*;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::collect::possible_resource_collections;
use server::content::custom_actions::CustomActionType;
use server::content::custom_phase_actions::{SelectedStructure, Structure};
use server::game::Game;
use server::playing_actions::PlayingActionType;
use server::position::Position;
use server::resource::ResourceType;
use server::unit::{UnitType, Units};
use std::collections::HashMap;
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
        wonder_icons(rc, city),
        custom_action_buttons(rc, Some(city)),
    ]
    .into_iter()
    .flatten()
    .collect();
    show_map_action_buttons(rc, &icons)
}

fn increase_happiness_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    if city.mood_state == MoodState::Happy || !can_play_increase_happiness(rc) {
        return None;
    }
    Some((
        &rc.assets().resources[&ResourceType::MoodTokens],
        "Increase happiness".to_string(),
        Box::new(move || {
            open_increase_happiness_dialog(rc, |mut happiness| {
                let mut target = city.mood_state.clone();
                while target != MoodState::Happy {
                    happiness = add_increase_happiness(rc, city, happiness);
                    target = target.clone().add(1);
                }
                happiness
            })
        }),
    ))
}

fn wonder_icons<'a>(rc: &'a RenderContext, city: &'a City) -> IconActionVec<'a> {
    if !city.can_activate() || !rc.can_play_action(PlayingActionType::Construct) {
        // is this the right thing to check?
        return vec![];
    }
    let owner = rc.shown_player;
    let game = rc.game;

    owner
        .wonder_cards
        .iter()
        .filter(|w| city.can_build_wonder(w, owner, game))
        .map(|w| {
            let a: IconAction<'a> = (
                &rc.assets().wonders[&w.name],
                format!("Build wonder {}", w.name),
                Box::new(move || {
                    StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(
                        ConstructionPayment::new(
                            rc,
                            city,
                            &w.name,
                            ConstructionProject::Wonder(w.name.clone()),
                        ),
                    ))
                }),
            );
            a
        })
        .collect()
}

fn building_icons<'a>(rc: &'a RenderContext, city: &'a City) -> IconActionVec<'a> {
    if !city.can_activate() || !rc.can_play_action(PlayingActionType::Construct) {
        return vec![];
    }
    let owner = rc.shown_player;
    Building::all()
        .iter()
        .filter_map(|b| {
            if city.can_construct(*b, owner, rc.game) {
                Some(*b)
            } else {
                None
            }
        })
        .flat_map(|b| new_building_positions(b, rc, city))
        .map(|(b, pos)| {
            let name = b.name();
            let tooltip = format!(
                "Built {}{} for {}",
                name,
                pos.map_or(String::new(), |p| format!(" at {p}")),
                owner.construct_cost(b, city, None).cost,
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
                        ),
                    ))
                }),
            );
            a
        })
        .collect()
}

fn recruit_button<'a>(rc: &'a RenderContext, city: &'a City) -> Option<IconAction<'a>> {
    if !city.can_activate() || !rc.can_play_action(PlayingActionType::Recruit) {
        return None;
    }
    Some((
        &rc.assets().units[&UnitType::Infantry],
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
    if !city.can_activate()
        || !base_or_custom_available(
            rc,
            PlayingActionType::Collect,
            &CustomActionType::FreeEconomyCollect,
        )
    {
        return None;
    }
    Some((
        &rc.assets().resources[&ResourceType::Food],
        "Collect Resources".to_string(),
        Box::new(|| {
            base_or_custom_action(
                rc,
                PlayingActionType::Collect,
                "Collect resources",
                &[("Free Economy", CustomActionType::FreeEconomyCollect)],
                |custom| {
                    let i = possible_resource_collections(
                        rc.game,
                        city.position,
                        city.player_index,
                        &HashMap::new(),
                        &[],
                    );
                    ActiveDialog::CollectResources(CollectResources::new(
                        city.player_index,
                        city.position,
                        custom,
                        i,
                    ))
                },
            )
        }),
    ))
}

pub fn city_labels(game: &Game, city: &City) -> Vec<String> {
    [
        vec![format!(
            "City: {}, {}, {} {}",
            game.get_player(city.player_index).get_name(),
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
                        format!("{} (owned by {})", b.name(), game.get_player(*o).get_name())
                    }
                })
            })
            .collect(),
    ]
    .concat()
}

pub const BUILDING_SIZE: f32 = 12.0;

fn structure_selected(
    rc: &RenderContext,
    position: Position,
    center: Vec2,
    size: f32,
    h: &StructureHighlight,
) -> Option<StateUpdate> {
    draw_circle_lines(center.x, center.y, size, 3., h.highlight_type.color());
    if is_mouse_button_pressed(MouseButton::Left) && is_in_circle(rc.mouse_pos(), center, size) {
        let ActiveDialog::StructuresRequest(r) = &rc.state.active_dialog else {
            panic!("invalid state");
        };
        Some(StateUpdate::OpenDialog(ActiveDialog::StructuresRequest(
            r.clone().toggle((position, h.structure.clone())),
        )))
    } else {
        None
    }
}

pub fn draw_city(rc: &RenderContext, city: &City) -> Option<StateUpdate> {
    let c = hex_ui::center(city.position);
    let owner = city.player_index;

    let highlighted = match rc.state.active_dialog {
        ActiveDialog::StructuresRequest(ref s) => highlight_structures(
            &position_structures(city, &s.selected),
            HighlightType::Primary,
        )
        .into_iter()
        .chain(highlight_structures(
            &position_structures(city, &s.request.choices),
            HighlightType::Choices,
        ))
        .collect_vec(),
        _ => vec![],
    };

    if let Some(h) = highlighted
        .iter()
        .find(|s| matches!(s.structure, Structure::CityCenter))
    {
        if let Some(u) = structure_selected(rc, city.position, c, 15., h) {
            return Some(u);
        }
    } else if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, rc.player_color(owner));

    let state = &rc.state;
    let mood = if let ActiveDialog::IncreaseHappiness(increase) = &state.active_dialog {
        let steps = increase
            .steps
            .iter()
            .find(|(p, _)| p == &city.position)
            .map_or(&0, |(_, s)| s);
        &city.mood_state.clone().add(*steps)
    } else {
        &city.mood_state
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

    let i = match draw_wonders(rc, city, c, owner, &highlighted) {
        Ok(value) => value,
        Err(value) => return Some(value),
    };

    draw_buildings(rc, city, c, &highlighted, i)
}

fn draw_buildings(
    rc: &RenderContext,
    city: &City,
    center: Vec2,
    highlighted: &[StructureHighlight],
    mut i: usize,
) -> Option<StateUpdate> {
    let state = &rc.state;
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let p = building_position(city, center, i, *b);
            if let Some(h) = highlighted
                .iter()
                .find(|s| matches!(s.structure, Structure::Building(bb) if bb == *b))
            {
                if let Some(u) = structure_selected(rc, city.position, p, BUILDING_SIZE, h) {
                    return Some(u);
                }
            }
            draw_circle(p.x, p.y, BUILDING_SIZE, rc.player_color(player_index));
            let tooltip = if matches!(state.active_dialog, ActiveDialog::CulturalInfluence(_)) {
                ""
            } else {
                b.name()
            };
            draw_scaled_icon(
                rc,
                &rc.assets().buildings[b],
                tooltip,
                p + vec2(-8., -8.),
                16.,
            );
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
    highlighted: &[StructureHighlight],
) -> Result<usize, StateUpdate> {
    let mut i = 0;
    for w in &city.pieces.wonders {
        let p = hex_ui::rotate_around(c, 20.0, 90 * i);
        draw_circle(p.x, p.y, 18.0, rc.player_color(owner));
        let size = 20.;
        if let Some(h) = highlighted
            .iter()
            .find(|s| matches!(&s.structure, Structure::Wonder(n) if n == &w.name))
        {
            if let Some(u) = structure_selected(rc, city.position, p, 18., h) {
                return Err(u);
            }
        }
        draw_scaled_icon(
            rc,
            &rc.assets().wonders[&w.name],
            &w.name,
            p + vec2(-size / 2., -size / 2.),
            size,
        );
        i += 1;
    }
    Ok(i)
}

fn position_structures(city: &City, list: &[SelectedStructure]) -> Vec<Structure> {
    list.iter()
        .filter_map(|(pos, st)| {
            if pos == &city.position {
                Some(st.clone())
            } else {
                None
            }
        })
        .collect_vec()
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
