use crate::action_buttons::base_or_custom_available;
use crate::city_ui::{building_position, BUILDING_SIZE};
use crate::client_state::{ActiveDialog, CameraMode, StateUpdate};
use crate::custom_phase_ui::SelectedStructureWithInfo;
use crate::dialog_ui::{BaseOrCustomAction, BaseOrCustomDialog};
use crate::hex_ui;
use crate::layout_ui::is_in_circle;
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_circle;
use itertools::Itertools;
use macroquad::input::{is_mouse_button_pressed, MouseButton};
use macroquad::math::Vec2;
use macroquad::prelude::{draw_circle_lines, WHITE};
use server::action::Action;
use server::city::City;
use server::content::custom_actions::{CustomAction, CustomActionType};
use server::content::custom_phase_actions::{SelectedStructure, Structure};
use server::cultural_influence::influence_culture_boost_cost;
use server::game::Game;
use server::player::Player;
use server::player_events::InfluenceCulturePossible;
use server::playing_actions::{ PlayingAction, PlayingActionType};
use server::position::Position;

pub fn hover(rc: &RenderContext, mouse_pos: Vec2, b: &BaseOrCustomDialog) -> StateUpdate {
    for p in &rc.game.players {
        for city in &p.cities {
            if let Some(value) = show_city(rc, mouse_pos, city, b) {
                return value;
            }
        }
    }

    StateUpdate::None
}

pub fn new_cultural_influence_dialog(
    game: &Game,
    player: usize,
    d: BaseOrCustomDialog,
) -> ActiveDialog {
    let a = game
        .players
        .iter()
        .flat_map(|p| {
            p.cities
                .iter()
                .flat_map(|city| {
                    structures(city)
                        .iter()
                        .filter_map(|s| {
                            let info = influence_culture_boost_cost(game, player, s);
                            if matches!(info.possible, InfluenceCulturePossible::Impossible) {
                                None
                            } else {
                                Some(SelectedStructureWithInfo::new(
                                    s.0,
                                    s.1.clone(),
                                    false,   //todo
                                    "".to_string(),   //todo
                                    "".to_string(),   //todo
                                ))
                            }
                        })
                        .collect_vec()
                })
                .collect_vec()
        })
        .collect_vec();

   ActiveDialog::StructuresRequest()
}

fn structures(city: &City) -> Vec<SelectedStructure> {
    let mut structures: Vec<SelectedStructure> = vec![(city.position, Structure::CityCenter)];
    for b in city.pieces.buildings(None) {
        structures.push((city.position, Structure::Building(*b)));
    }
    structures
}

pub fn can_play_influence_culture(rc: &RenderContext) -> bool {
    base_or_custom_available(
        rc,
        &PlayingActionType::InfluenceCultureAttempt,
        &CustomActionType::ArtsInfluenceCultureAttempt,
    )
}
