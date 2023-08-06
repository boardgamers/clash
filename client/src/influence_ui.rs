use crate::city_ui::CityMenu;
use macroquad::ui::Ui;
use server::city_pieces::Building;
use server::game::{Action, Game};
use server::playing_actions::PlayingAction;
use server::position::Position;

pub fn add_influence_button(
    game: &mut Game,
    menu: &CityMenu,
    ui: &mut Ui,
    closest_city_pos: &Position,
    building: &Building,
    building_name: &str,
) {
    if !menu.get_city(game).city_pieces.can_add_building(building) {
        let start_position = if menu.is_city_owner() {
            menu.city_position
        } else {
            closest_city_pos
        };
        if let Some(cost) = game.influence_culture_boost_cost(
            menu.player_index,
            start_position,
            menu.city_owner_index,
            menu.city_position,
            building,
        ) {
            if ui.button(
                None,
                format!("Attempt Influence {} for {}", building_name, cost),
            ) {
                game.execute_action(
                    Action::PlayingAction(PlayingAction::InfluenceCultureAttempt {
                        starting_city_position: start_position.clone(),
                        target_player_index: menu.city_owner_index,
                        target_city_position: menu.city_position.clone(),
                        city_piece: building.clone(),
                    }),
                    menu.player_index,
                );
            }
        }
    }
}

pub fn closest_city(game: &&mut Game, menu: &CityMenu) -> Position {
    menu.get_player(game)
        .cities
        .iter()
        .min_by_key(|c| c.position.distance(menu.city_position))
        .unwrap()
        .position
        .clone()
}
