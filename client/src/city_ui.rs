use macroquad::prelude::*;

use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::{UnitType, Units};

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate};
use crate::collect_ui::{possible_resource_collections, CollectResources};
use crate::construct_ui::{building_positions, ConstructionPayment, ConstructionProject};
use crate::layout_ui::{bottom_center_texture, draw_scaled_icon, icon_pos};
use crate::recruit_unit_ui::RecruitAmount;
use crate::resource_ui::ResourceType;
use crate::{hex_ui, player_ui};

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

pub type IconActionVec<'a> = Vec<(&'a Texture2D, String, Box<dyn Fn() -> StateUpdate + 'a>)>;

pub fn show_city_menu<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> StateUpdate {
    let city = menu.get_city(game);

    let can_play = menu.player.can_play_action && menu.is_city_owner() && city.can_activate();
    if !can_play {
        return StateUpdate::None;
    }
    let mut icons: IconActionVec<'a> = vec![];
    icons.push((
        &state.assets.resources[&ResourceType::Food],
        "Collect Resources".to_string(),
        Box::new(|| {
            StateUpdate::SetDialog(ActiveDialog::CollectResources(CollectResources::new(
                menu.player.index,
                menu.city_position,
                possible_resource_collections(game, menu.city_position, menu.city_owner_index),
            )))
        }),
    ));
    icons.push((
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
    ));

    let owner = menu.get_city_owner(game);
    let city = menu.get_city(game);
    // let closest_city_pos = influence_ui::closest_city(game, menu);

    for (building, name) in building_names() {
        if menu.is_city_owner()
            && menu.player.can_play_action
            && city.can_construct(building, owner)
        {
            for pos in building_positions(building, city, &game.map) {
                let tooltip = format!(
                    "Built {}{} for {}",
                    name,
                    pos.map_or(String::new(), |p| format!(" at {p}")),
                    owner.construct_cost(building, city),
                );
                icons.push((
                    &state.assets.buildings[&building],
                    tooltip,
                    Box::new(move || {
                        StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(
                            ConstructionPayment::new(
                                game,
                                name,
                                menu.player.index,
                                menu.city_position,
                                ConstructionProject::Building(building, pos),
                            ),
                        ))
                    }),
                ));
            }
        }
        // todo should not be in city menu
        // updates.add(influence_ui::add_influence_button(
        //     game,
        //     menu,
        //     ui,
        //     closest_city_pos,
        //     &building,
        //     name,
        // ));
    }

    for w in &owner.wonder_cards {
        if city.can_build_wonder(w, owner, game) {
            icons.push((
                &state.assets.wonders[&w.name],
                format!("Build wonder {}", w.name),
                Box::new(move || {
                    StateUpdate::SetDialog(ActiveDialog::ConstructionPayment(
                        ConstructionPayment::new(
                            game,
                            &w.name,
                            menu.player.index,
                            menu.city_position,
                            ConstructionProject::Wonder(w.name.clone()),
                        ),
                    ))
                }),
            ));
        }
    }

    for (i, (icon, tooltip, action)) in icons.iter().enumerate() {
        if bottom_center_texture(
            state,
            icon,
            icon_pos(-(icons.len() as i8) / 2 + i as i8, -1),
            tooltip,
        ) {
            return action();
        }
    }
    StateUpdate::None
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
                        building_name(b).to_string()
                    } else {
                        format!(
                            "{} (owned by {})",
                            building_name(b),
                            game.get_player(*o).get_name()
                        )
                    }
                })
            })
            .collect(),
    ]
    .concat()
}

pub fn draw_city(owner: &Player, city: &City, state: &State) {
    let c = hex_ui::center(city.position);

    if city.is_activated() {
        draw_circle(c.x, c.y, 18.0, WHITE);
    }
    draw_circle(c.x, c.y, 15.0, player_ui::player_color(owner.index));

    if let ActiveDialog::IncreaseHappiness(increase) = &state.active_dialog {
        let steps = increase
            .steps
            .iter()
            .find(|(p, _)| p == &city.position)
            .map_or(String::new(), |(_, s)| format!("{s}"));
        state.draw_text(&steps, c.x - 5., c.y + 6.);
    } else {
        let t = match city.mood_state {
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
            let p = if matches!(b, Building::Port) {
                let r: f32 = city
                    .position
                    .coordinate()
                    .directions_to(city.port_position.unwrap().coordinate())[0]
                    .to_radians_pointy();
                hex_ui::rotate_around_rad(c, 60.0, r * -1.0 + std::f32::consts::PI / 3.0)
            } else {
                hex_ui::rotate_around(c, 25.0, 90 * i)
            };
            draw_circle(p.x, p.y, 12.0, player_ui::player_color(player_index));
            draw_scaled_icon(
                state,
                &state.assets.buildings[b],
                building_name(b),
                p.to_vec2() + vec2(-8., -8.),
                16.,
            );
            i += 1;
        }
    }
}

pub fn building_name(b: &Building) -> &str {
    building_names()
        .iter()
        .find_map(|(b2, n)| if b == b2 { Some(n) } else { None })
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
