use crate::city_ui::{building_name, building_position, BUILDING_SIZE};
use crate::client_state::{CameraMode, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button, OkTooltip};
use crate::hex_ui;
use crate::layout_ui::is_in_circle;
use crate::render_context::RenderContext;
use crate::resource_ui::{show_resource_pile, ResourceType};
use crate::tooltip::show_tooltip_for_circle;
use macroquad::input::{is_mouse_button_pressed, MouseButton};
use macroquad::math::Vec2;
use macroquad::prelude::{draw_circle_lines, WHITE};
use server::action::Action;
use server::city::City;
use server::game::CulturalInfluenceResolution;
use server::player::Player;
use server::playing_actions::{InfluenceCultureAttempt, PlayingAction};
use server::position::Position;
use server::resource_pile::ResourcePile;

fn closest_city(player: &Player, position: Position) -> Position {
    player
        .cities
        .iter()
        .min_by_key(|c| c.position.distance(position))
        .unwrap()
        .position
}

pub fn cultural_influence_resolution_dialog(
    rc: &RenderContext,
    r: &CulturalInfluenceResolution,
) -> StateUpdate {
    let name = building_name(r.city_piece);
    let pile = ResourcePile::culture_tokens(r.roll_boost_cost);
    show_resource_pile(rc, &pile, &[ResourceType::CultureTokens]);
    if ok_button(rc, OkTooltip::Valid(format!("Influence {name} for {pile}"))) {
        return StateUpdate::Execute(Action::CulturalInfluenceResolution(true));
    }
    if cancel_button_with_tooltip(rc, "Decline") {
        return StateUpdate::Execute(Action::CulturalInfluenceResolution(false));
    }

    StateUpdate::None
}

pub fn hover(rc: &RenderContext, mouse_pos: Vec2) -> StateUpdate {
    for p in &rc.game.players {
        for city in &p.cities {
            if let Some(value) = show_city(rc, mouse_pos, city) {
                return value;
            }
        }
    }

    StateUpdate::None
}

fn show_city(rc: &RenderContext, mouse_pos: Vec2, city: &City) -> Option<StateUpdate> {
    let player = rc.shown_player;
    let c = hex_ui::center(city.position);
    let mut i = city.pieces.wonders.len() as i32;
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let center = building_position(city, c, i, *b);
            let closest_city_pos = closest_city(player, city.position);
            let start_position = if city.player_index == player.index {
                city.position
            } else {
                closest_city_pos
            };

            if let Some(cost) = rc.game.influence_culture_boost_cost(
                player.index,
                start_position,
                city.player_index,
                city.position,
                *b,
            ) {
                if player.resources.can_afford(&cost) {
                    let name = building_name(*b);
                    let _ = rc.with_camera(CameraMode::World, |rc| {
                        draw_circle_lines(center.x, center.y, BUILDING_SIZE, 1., WHITE);
                        show_tooltip_for_circle(
                            rc,
                            &format!("Attempt Influence {name} for {cost}"),
                            center.to_vec2(),
                            BUILDING_SIZE,
                        );
                        StateUpdate::None
                    });

                    if is_in_circle(mouse_pos, center, BUILDING_SIZE)
                        && is_mouse_button_pressed(MouseButton::Left)
                    {
                        return Some(StateUpdate::Execute(Action::Playing(
                            PlayingAction::InfluenceCultureAttempt(InfluenceCultureAttempt {
                                starting_city_position: start_position,
                                target_player_index: city.player_index,
                                target_city_position: city.position,
                                city_piece: *b,
                            }),
                        )));
                    }
                }
            }

            i += 1;
        }
    }
    None
}
