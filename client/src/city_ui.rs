use macroquad::prelude::*;
use macroquad::ui::Ui;

use server::city::{City, MoodState};
use server::city_pieces::Building;
use server::game::Game;
use server::player::Player;
use server::position::Position;
use server::unit::Units;

use crate::client_state::{ActiveDialog, ShownPlayer, State, StateUpdate, StateUpdates};
use crate::collect_ui::{possible_resource_collections, CollectResources};
use crate::construct_ui::{add_construct_button, add_wonder_buttons};
use crate::hex_ui::draw_hex_center_text;
use crate::layout_ui::{bottom_center_texture, icon_pos};
use crate::recruit_unit_ui::RecruitAmount;
use crate::resource_ui::ResourceType;
use crate::{hex_ui, influence_ui, player_ui};

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

pub type IconActionVec<'a> = Vec<(&'a Texture2D, &'a str, Box<dyn Fn() -> StateUpdate + 'a>)>;

pub fn show_city_menu<'a>(game: &'a Game, menu: &'a CityMenu, state: &'a State) -> StateUpdate {
    let city = menu.get_city(game);

    let can_play = menu.player.can_play_action && menu.is_city_owner() && city.can_activate();
    if !can_play {
        return StateUpdate::None;
    }
    let mut icons: IconActionVec<'a> = vec![];
    icons.push((
        &state.assets.resources[&ResourceType::Food],
        "Collect Resources",
        Box::new(|| {
            StateUpdate::SetDialog(ActiveDialog::CollectResources(CollectResources::new(
                menu.player.index,
                menu.city_position,
                possible_resource_collections(game, menu.city_position, menu.city_owner_index),
            )))
        }),
    ));
    icons.push((
        &state.assets.warrior,
        "Recruit Units",
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

    add_building_actions(game, state, menu, &mut icons);
    // add_wonder_buttons(game, menu, &mut icons);

    for (i, (icon, tooltip, action)) in icons.iter().enumerate() {
        if bottom_center_texture(
            state,
            icon,
            icon_pos(-1 * icons.len() as i8 / 2 + i as i8, -1),
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

fn add_building_actions(game: &Game, state: &State, menu: &CityMenu, icons: &mut IconActionVec) {
    let closest_city_pos = influence_ui::closest_city(game, menu);

    for (building, name) in building_names() {
        add_construct_button(game, state, menu, icons, &building, name);
        // todo
        // updates.add(influence_ui::add_influence_button(
        //     game,
        //     menu,
        //     ui,
        //     closest_city_pos,
        //     &building,
        //     name,
        // ));
    }
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
        draw_hex_center_text(city.position, &steps);
    } else {
        match city.mood_state {
            MoodState::Happy => draw_hex_center_text(city.position, "+"),
            MoodState::Neutral => {}
            MoodState::Angry => draw_hex_center_text(city.position, "-"),
        }
    }

    let mut i = 0;
    city.pieces.wonders.iter().for_each(|w| {
        let p = hex_ui::rotate_around(c, 20.0, 90 * i);
        draw_circle(p.x, p.y, 18.0, player_ui::player_color(owner.index));
        draw_text(&w.name, p.x - 10.0, p.y + 10.0, 40.0, BLACK);
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
                hex_ui::rotate_around(c, 20.0, 90 * i)
            };
            draw_circle(p.x, p.y, 12.0, player_ui::player_color(player_index));
            draw_text(building_symbol(b), p.x - 7.0, p.y + 8.0, 30.0, BLACK);
            i += 1;
        }
    }
}

fn building_symbol(b: &Building) -> &str {
    match b {
        Building::Academy => "A",
        Building::Market => "M",
        Building::Obelisk => "K",
        Building::Observatory => "V",
        Building::Fortress => "F",
        Building::Port => "P",
        Building::Temple => "T",
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
