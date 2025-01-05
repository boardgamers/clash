use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::collect_ui::{possible_resource_collections, CollectResources};
use crate::construct_ui::{new_building_positions, ConstructionPayment, ConstructionProject};
use crate::happiness_ui::{add_increase_happiness, IncreaseHappiness};
use crate::hex_ui::Point;
use crate::layout_ui::draw_scaled_icon;
use crate::map_ui::{move_units_button, show_map_action_buttons};
use crate::recruit_unit_ui::RecruitAmount;
use crate::resource_ui::ResourceType;
use crate::{hex_ui, player_ui};
use macroquad::prelude::*;
use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{UnitType, Units};
use std::ops::Add;

pub struct CityMenu {
    pub player: ShownPlayer,
    pub city_owner_index: usize,
    pub city_position: Position,
}

impl CityMenu {
    pub fn new(player: &ShownPlayer, city_owner_index: usize, city_position: Position) -> Self {
        CityMenu {
            player: player.clone(),
            city_owner_index,
            city_position,
        }
    }

    pub fn get_city_owner<'a>(&self, game: &'a Game) -> &'a Player {
        game.get_player(self.city_owner_index)
    }

    pub fn get_city<'a>(&self, game: &'a Game) -> &'a City {
        game.get_city(self.city_owner_index, self.city_position)
    }

    pub fn is_city_owner(&self) -> bool {
        self.player.index == self.city_owner_index
    }
}

pub type IconAction<'a> = (&'a Texture2D, String, Box<dyn Fn() -> StateUpdate + 'a>);

pub type IconActionVec<'a> = Vec<IconAction<'a>>;

pub fn show_city_menu<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> StateUpdate {
    let city = menu.get_city(game);
    let pos = menu.city_position;

    let can_play = menu.player.can_play_action && menu.is_city_owner() && city.can_activate();
    if !can_play {
        return StateUpdate::None;
    }

    let base_icons: IconActionVec<'a> = vec![
        increase_happiness_button(game, menu, state),
        move_units_button(game, pos, &menu.player, state),
        Some(collect_resources_button(game, menu, state)),
        Some(recruit_button(game, menu, state)),
    ]
    .into_iter()
    .flatten()
    .collect();

    let buildings: IconActionVec<'a> = building_icons(game, menu, state);

    let wonders: IconActionVec<'a> = wonder_icons(game, menu, state);

    show_map_action_buttons(
        state,
        &vec![base_icons, buildings, wonders]
            .into_iter()
            .flatten()
            .collect(),
    )
}

fn increase_happiness_button<'a>(
    game: &'a Game,
    menu: &'a CityMenu,
    state: &'a State,
) -> Option<IconAction<'a>> {
    let city = menu.get_city(game);
    if city.mood_state == MoodState::Happy {
        return None;
    }
    Some((
        &state.assets.resources[&ResourceType::MoodTokens],
        "Increase happiness".to_string(),
        Box::new(move || {
            let player = &menu.player;
            let mut happiness = IncreaseHappiness::new(player.get(game));
            let mut target = city.mood_state.clone();
            while target != MoodState::Happy {
                happiness = add_increase_happiness(city, &happiness);
                target = target.clone().add(1);
            }
            StateUpdate::OpenDialog(ActiveDialog::IncreaseHappiness(happiness))
        }),
    ))
}

fn wonder_icons<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> IconActionVec<'a> {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);

    owner
        .wonder_cards
        .iter()
        .filter(|w| city.can_build_wonder(w, owner, game))
        .map(|w| {
            let a: IconAction<'a> = (
                &state.assets.wonders[&w.name],
                format!("Build wonder {}", w.name),
                Box::new(move || {
                    StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(
                        ConstructionPayment::new(
                            game,
                            &w.name,
                            menu.player.index,
                            menu.city_position,
                            ConstructionProject::Wonder(w.name.clone()),
                        ),
                    ))
                }),
            );
            a
        })
        .collect()
}

fn building_icons<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> IconActionVec<'a> {
    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    building_names()
        .iter()
        .filter_map(|(b, _)| {
            if menu.is_city_owner() && menu.player.can_play_action && city.can_construct(*b, owner)
            {
                Some(*b)
            } else {
                None
            }
        })
        .flat_map(|b| new_building_positions(b, city, &game.map))
        .map(|(b, pos)| {
            let name = building_name(b);
            let tooltip = format!(
                "Built {}{} for {}",
                name,
                pos.map_or(String::new(), |p| format!(" at {p}")),
                owner.construct_cost(b, city),
            );
            let a: IconAction<'a> = (
                &state.assets.buildings[&b],
                tooltip,
                Box::new(move || {
                    StateUpdate::OpenDialog(ActiveDialog::ConstructionPayment(
                        ConstructionPayment::new(
                            game,
                            name,
                            menu.player.index,
                            menu.city_position,
                            ConstructionProject::Building(b, pos),
                        ),
                    ))
                }),
            );
            a
        })
        .collect()
}

fn recruit_button<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> IconAction<'a> {
    (
        &state.assets.units[&UnitType::Infantry],
        "Recruit Units".to_string(),
        Box::new(|| {
            RecruitAmount::new_selection(
                game,
                menu.player.index,
                menu.city_position,
                Units::empty(),
                None,
                &[],
            )
        }),
    )
}

fn collect_resources_button<'a>(
    game: &'a Game,
    menu: &'a CityMenu,
    state: &'a State,
) -> IconAction<'a> {
    (
        &state.assets.resources[&ResourceType::Food],
        "Collect Resources".to_string(),
        Box::new(|| {
            let pos = menu.city_position;
            StateUpdate::OpenDialog(ActiveDialog::CollectResources(CollectResources::new(
                menu.player.index,
                pos,
                possible_resource_collections(game, pos, menu.city_owner_index),
            )))
        }),
    )
}

pub fn city_labels(game: &Game, city: &City) -> Vec<String> {
    [
        vec![format!(
            "Size: {} Mood: {} Activated: {}",
            city.size(),
            match city.mood_state {
                MoodState::Happy => "Happy",
                MoodState::Neutral => "Neutral",
                MoodState::Angry => "Angry",
            },
            city.is_activated()
        )],
        city.pieces
            .building_owners()
            .iter()
            .filter_map(|(b, o)| {
                o.as_ref().map(|o| {
                    if city.player_index == *o {
                        building_name(*b).to_string()
                    } else {
                        format!(
                            "{} (owned by {})",
                            building_name(*b),
                            game.get_player(*o).get_name()
                        )
                    }
                })
            })
            .collect(),
    ]
    .concat()
}

pub const BUILDING_SIZE: f32 = 12.0;

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    let c = hex_ui::center(city.position);

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, player_ui::player_color(owner.index));

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
        MoodState::Happy => Some(&state.assets.resources[&ResourceType::MoodTokens]),
        MoodState::Neutral => None,
        MoodState::Angry => Some(&state.assets.angry),
    };
    if let Some(t) = t {
        let size = 15.;
        draw_scaled_icon(
            state,
            t,
            &format!("Happiness: {:?}", city.mood_state),
            c.to_vec2() + vec2(-size / 2., -size / 2.),
            size,
        );
    }

    let mut i = 0;
    city.pieces.wonders.iter().for_each(|w| {
        let p = hex_ui::rotate_around(c, 20.0, 90 * i);
        draw_circle(p.x, p.y, 18.0, player_ui::player_color(owner.index));
        let size = 20.;
        draw_scaled_icon(
            state,
            &state.assets.wonders[&w.name],
            &w.name,
            p.to_vec2() + vec2(-size / 2., -size / 2.),
            size,
        );
        i += 1;
    });

    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let p = building_position(city, c, i, *b);
            draw_circle(
                p.x,
                p.y,
                BUILDING_SIZE,
                player_ui::player_color(player_index),
            );
            let tooltip = if matches!(state.active_dialog, ActiveDialog::CulturalInfluence) {
                ""
            } else {
                building_name(*b)
            };
            draw_scaled_icon(
                state,
                &state.assets.buildings[b],
                tooltip,
                p.to_vec2() + vec2(-8., -8.),
                16.,
            );
            i += 1;
        }
    }
}

pub fn building_position(city: &City, center: Point, i: i32, building: Building) -> Point {
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

pub fn building_name(b: Building) -> &'static str {
    building_names()
        .iter()
        .find_map(|(b2, n)| if &b == b2 { Some(n) } else { None })
        .unwrap()
}

fn building_names() -> [(Building, &'static str); 7] {
    [
        (Building::Academy, "Academy"),
        (Building::Market, "Market"),
        (Building::Obelisk, "Obelisk"),
        (Building::Observatory, "Observatory"),
        (Building::Fortress, "Fortress"),
        (Building::Port, "Port"),
        (Building::Temple, "Temple"),
    ]
}
