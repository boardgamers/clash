use crate::city_ui::{building_name, building_position, BUILDING_SIZE};
use crate::client_state::{ShownPlayer, State, StateUpdate};
use crate::dialog_ui::active_dialog_window;
use crate::hex_ui;
use crate::layout_ui::is_in_circle;
use crate::tooltip::show_tooltip_for_circle;
use macroquad::input::{is_mouse_button_pressed, MouseButton};
use macroquad::math::Vec2;
use server::action::Action;
use server::game::{CulturalInfluenceResolution, Game};
use server::player::Player;
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

pub fn cultural_influence_resolution_dialog(
    c: &CulturalInfluenceResolution,
    player: &ShownPlayer,
) -> StateUpdate {
    active_dialog_window(player, "Cultural Influence Resolution", |ui| {
        if ui.button(
            None,
            format!(
                "Pay {} culture tokens to influence {}",
                c.roll_boost_cost,
                building_name(&c.city_piece)
            ),
        ) {
            StateUpdate::Execute(Action::CulturalInfluenceResolution(true))
        } else if ui.button(None, "Decline") {
            StateUpdate::Execute(Action::CulturalInfluenceResolution(false))
        } else {
            StateUpdate::None
        }
    })
}

pub fn hover(
    position: Position,
    game: &Game,
    shown_player: &ShownPlayer,
    mouse_pos: Vec2,
    state: &mut State,
) -> StateUpdate {
    let player = game.get_player(shown_player.index);
    if let Some(city) = game.get_any_city(position) {
        let c = hex_ui::center(city.position);
        let mut i = city.pieces.wonders.len() as i32;
        for player_index in 0..4 {
            for b in &city.pieces.buildings(Some(player_index)) {
                let center = building_position(city, c, i, *b);

                if is_in_circle(mouse_pos, center, BUILDING_SIZE) {
                    let closest_city_pos = closest_city(player, position);

                    let start_position = if city.player_index == shown_player.index {
                        city.position
                    } else {
                        closest_city_pos
                    };
                    if let Some(cost) = game.influence_culture_boost_cost(
                        shown_player.index,
                        start_position,
                        city.player_index,
                        city.position,
                        *b,
                    ) {
                        let player = game.get_player(shown_player.index);
                        if player.resources.can_afford(&cost) {
                            if is_mouse_button_pressed(MouseButton::Left) {
                                return StateUpdate::Execute(Action::Playing(
                                    PlayingAction::InfluenceCultureAttempt(
                                        InfluenceCultureAttempt {
                                            starting_city_position: start_position,
                                            target_player_index: city.player_index,
                                            target_city_position: city.position,
                                            city_piece: *b,
                                        },
                                    ),
                                ));
                            }
                            let name = building_name(b);
                            state.set_world_camera();
                            show_tooltip_for_circle(
                                state,
                                &format!("Attempt Influence {name} for {cost}"),
                                center.to_vec2(),
                                BUILDING_SIZE,
                            );
                            state.set_screen_camera();
                        }
                    }
                }

                i += 1;
            }
        }
    }
    StateUpdate::None
}
