use crate::city_ui::{building_name, building_position, BUILDING_SIZE};
use crate::client_state::{ShownPlayer, State, StateUpdate};
use crate::dialog_ui::{cancel_button_with_tooltip, ok_button};
use crate::hex_ui;
use crate::layout_ui::is_in_circle;
use crate::tooltip::show_tooltip_for_circle;
use macroquad::input::{is_mouse_button_pressed, MouseButton};
use macroquad::math::Vec2;
use macroquad::prelude::{draw_circle_lines, WHITE};
use server::action::Action;
use server::city::City;
use server::game::Game;
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

pub fn cultural_influence_resolution_dialog(state: &State) -> StateUpdate {
    if ok_button(state, true) {
        return StateUpdate::Execute(Action::CulturalInfluenceResolution(true));
    }
    if cancel_button_with_tooltip(state, "Decline") {
        return StateUpdate::Execute(Action::CulturalInfluenceResolution(false));
    }

    StateUpdate::None
}

pub fn hover(
    position: Position,
    game: &Game,
    shown_player: &ShownPlayer,
    mouse_pos: Vec2,
    state: &mut State,
) -> StateUpdate {
    let player = shown_player.get(game);
    for p in &game.players {
        for city in &p.cities {
            if let Some(value) =
                show_city(position, game, shown_player, mouse_pos, state, player, city)
            {
                return value;
            }
        }
    }

    StateUpdate::None
}

fn show_city(
    position: Position,
    game: &Game,
    shown_player: &ShownPlayer,
    mouse_pos: Vec2,
    state: &mut State,
    player: &Player,
    city: &City,
) -> Option<StateUpdate> {
    let c = hex_ui::center(city.position);
    let mut i = city.pieces.wonders.len() as i32;
    for player_index in 0..4 {
        for b in &city.pieces.buildings(Some(player_index)) {
            let center = building_position(city, c, i, *b);
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
                if player.resources.can_afford(&cost) {
                    let name = building_name(b);
                    state.set_world_camera();
                    draw_circle_lines(center.x, center.y, BUILDING_SIZE, 1., WHITE);
                    show_tooltip_for_circle(
                        state,
                        &format!("Attempt Influence {name} for {cost}"),
                        center.to_vec2(),
                        BUILDING_SIZE,
                    );
                    state.set_screen_camera();

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
