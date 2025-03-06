use crate::city_ui::{building_position, BUILDING_SIZE};
use crate::client_state::{CameraMode, StateUpdate};
use crate::dialog_ui::{BaseOrCustomAction, BaseOrCustomDialog};
use crate::hex_ui;
use crate::layout_ui::is_in_circle;
use crate::render_context::RenderContext;
use crate::tooltip::show_tooltip_for_circle;
use macroquad::input::{is_mouse_button_pressed, MouseButton};
use macroquad::math::Vec2;
use macroquad::prelude::{draw_circle_lines, WHITE};
use server::action::Action;
use server::city::City;
use server::content::custom_actions::CustomAction;
use server::cultural_influence::influence_culture_boost_cost;
use server::player::Player;
use server::player_events::InfluenceCulturePossible;
use server::playing_actions::{InfluenceCultureAttempt, PlayingAction};
use server::position::Position;

fn closest_city(player: &Player, position: Position) -> Position {
    player
        .cities
        .iter()
        .min_by_key(|c| c.position.distance(position))
        .unwrap()
        .position
}

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

fn show_city(
    rc: &RenderContext,
    mouse_pos: Vec2,
    city: &City,
    custom: &BaseOrCustomDialog,
) -> Option<StateUpdate> {
    let player = rc.shown_player;
    let c = hex_ui::center(city.position);
    let mut i = city.pieces.wonders.len();
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let center = building_position(city, c, i, *b);
            let closest_city_pos = closest_city(player, city.position);
            let start_position = if city.player_index == player.index {
                city.position
            } else {
                closest_city_pos
            };

            let info = influence_culture_boost_cost(
                rc.game,
                player.index,
                start_position,
                city.player_index,
                city.position,
                *b,
            );
            if !matches!(info.possible, InfluenceCulturePossible::Impossible) {
                let name = b.name();
                let _ = rc.with_camera(CameraMode::World, |rc| {
                    draw_circle_lines(center.x, center.y, BUILDING_SIZE, 1., WHITE);
                    show_tooltip_for_circle(
                        rc,
                        &format!("Attempt Influence {name} for {}", info.range_boost_cost),
                        center,
                        BUILDING_SIZE,
                    );
                    StateUpdate::None
                });

                if is_in_circle(mouse_pos, center, BUILDING_SIZE)
                    && is_mouse_button_pressed(MouseButton::Left)
                {
                    let attempt = InfluenceCultureAttempt {
                        starting_city_position: start_position,
                        target_player_index: city.player_index,
                        target_city_position: city.position,
                        city_piece: *b,
                    };
                    let action = match custom.custom {
                        BaseOrCustomAction::Base => PlayingAction::InfluenceCultureAttempt(attempt),
                        BaseOrCustomAction::Custom { .. } => PlayingAction::Custom(
                            CustomAction::ArtsInfluenceCultureAttempt(attempt),
                        ),
                    };

                    return Some(StateUpdate::Execute(Action::Playing(action)));
                }
            }

            i += 1;
        }
    }
    None
}
